use reqwest::{
    header::{HeaderValue, COOKIE, REFERER, SET_COOKIE, USER_AGENT},
    Client, ClientBuilder,
};
use std::time::Duration;

/// Default user agent fallback if random UA is disabled
const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub struct Bypasser<'a> {
    wait: u8,
    retry: u32,
    proxy: Option<&'a str>,
    user_agent: String,
    client: Client,
    random_ua: bool,
}

impl<'a> Default for Bypasser<'a> {
    fn default() -> Self {
        Bypasser {
            wait: 3,
            retry: 1,
            proxy: None,
            user_agent: String::new(),
            client: ClientBuilder::new()
                .danger_accept_invalid_certs(true)
                .danger_accept_invalid_hostnames(true)
                .gzip(true)
                .brotli(true)
                .build()
                .unwrap(),
            random_ua: false,
        }
    }
}

impl<'a> Bypasser<'a> {
    pub fn wait(mut self, secs: u8) -> Self {
        self.wait = secs;
        self
    }

    pub fn user_agent(mut self, user_agent: &str) -> Self {
        self.user_agent = user_agent.to_owned();
        self
    }

    /// Enable random user agent selection from fake_user_agent database
    pub fn random_user_agent(mut self, flag: bool) -> Self {
        self.random_ua = flag;
        self
    }

    pub fn proxy(mut self, address: &'a str) -> Self {
        self.proxy = Some(address);
        self
    }

    pub fn retry(mut self, times: u32) -> Self {
        self.retry = times;
        self
    }

    fn build_client(&mut self) -> &mut Self {
        let mut client_builder = ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true)
            .gzip(true)
            .brotli(true)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30));
        if let Some(address) = self.proxy {
            client_builder = client_builder.proxy(reqwest::Proxy::all(address).unwrap());
        }
        self.client = client_builder.build().unwrap();
        self
    }

    fn get_user_agent(&mut self) -> &str {
        if self.random_ua && self.user_agent.is_empty() {
            self.user_agent = fake_user_agent::get_rua().to_string();
        }
        if self.user_agent.is_empty() {
            DEFAULT_USER_AGENT
        } else {
            &self.user_agent
        }
    }

    fn parse_challenge(html: &str) -> Vec<(String, String)> {
        regex::Regex::new(r#"name="(r|jschl_vc|pass)"(?: [^<>]*)? value="(.+?)""#)
            .unwrap()
            .captures_iter(html)
            .map(|caps| (caps[1].to_owned(), caps[2].to_owned()))
            .collect()
    }

    fn parse_js(html: &str, domain: &str) -> String {
        use regex::Regex;

        let challenge = &Regex::new(
			r#"setTimeout\(function\(\)\{\s+(var s,t,o,p,b,r,e,a,k,i,n,g,f.+?\r?\n[\s\S]+?a\.value =.+?)\r?\n"#,
		)
		.unwrap()
		.captures(html)
		.unwrap()[1];
        let inner_html = if let Some(caps) =
            Regex::new(r#"<div(?: [^<>]*)? id="cf-dn.*?">([^<>]*)"#)
                .unwrap()
                .captures(html)
        {
            caps[1].to_owned()
        } else {
            String::new()
        };
        format!(
            r#"
                var document = {{
                    createElement: function () {{
                        return {{ firstChild: {{ href: "http://{}/" }} }}
                    }},
                    getElementById: function () {{
                        return {{"innerHTML": "{}"}};
                    }}
                }};
                {}; process.stdout.write(a.value);
            "#,
            domain, inner_html, challenge
        )
    }

    fn run_js(js: &str) -> String {
        String::from_utf8(
            std::process::Command::new("node")
                .args(["-e", js])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
    }

    async fn request_challenge(&mut self, url: &str) -> (String, String, HeaderValue, String) {
        self.build_client();
        let ua = self.get_user_agent().to_string();
        let re = regex::Regex::new(r#"id="challenge-form" action="([^"]*)""#).unwrap();

        loop {
            match self.client.get(url).header(USER_AGENT, &ua).send().await {
                Ok(resp) => {
                    let url = resp.url().as_str().to_owned();
                    let cookie = resp.headers()[SET_COOKIE].to_owned();
                    match resp.text().await {
                        Ok(text) => {
                            let path = re.captures(&text).unwrap()[1].into();
                            return (text, url, cookie, path);
                        }
                        Err(e) => eprintln!("At request_challenge() text(), {:?}", e),
                    }
                }
                Err(e) => eprintln!("At request_challenge() send(), {:?}", e),
            }
        }
    }

    async fn solve_challenge(
        &mut self,
        url: &str,
        cookie: &HeaderValue,
        referer: &str,
        params: &[(String, String)],
    ) -> Result<(HeaderValue, HeaderValue), &str> {
        let ua = self.get_user_agent().to_string();
        let mut retry = 0u32;

        loop {
            match self
                .client
                .post(url)
                .header(COOKIE, cookie)
                .header(REFERER, referer)
                .header(USER_AGENT, &ua)
                .form(params)
                .send()
                .await
            {
                Ok(resp) => {
                    if resp.headers().contains_key(SET_COOKIE) {
                        return Ok((resp.headers()[SET_COOKIE].to_owned(), ua.parse().unwrap()));
                    }
                }
                Err(e) => eprintln!("{:?}", e),
            }

            retry += 1;
            if retry == self.retry {
                return Err("reach max retries");
            }
        }
    }

    /// Bypass Cloudflare protection (async version)
    ///
    /// Returns a tuple of (cookie HeaderValue, user-agent HeaderValue)
    /// that can be used in subsequent requests.
    pub async fn bypass(&mut self, url: &str) -> Result<(HeaderValue, HeaderValue), &str> {
        let (html, referer, cookie, path) = self.request_challenge(url).await;

        let (challenge_url, domain) = {
            let url = url::Url::parse(url).unwrap();
            let domain = url.domain().unwrap().to_owned();

            (format!("{}://{}{}", url.scheme(), domain, path), domain)
        };
        let params = {
            let mut p = Bypasser::parse_challenge(&html);
            p.push((
                String::from("jschl_answer"),
                Bypasser::run_js(&Bypasser::parse_js(&html, &domain)),
            ));

            p
        };

        tokio::time::sleep(Duration::from_secs(self.wait as u64)).await;

        self.solve_challenge(&challenge_url, &cookie, &referer, &params)
            .await
    }
}
