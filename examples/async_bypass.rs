use cloudflare_bypasser::Bypasser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    const WEBSITE: &str = "https://example.com"; // Replace with actual Cloudflare-protected site

    // Customize the bypasser
    let mut bypasser = Bypasser::default()
        .retry(30) // retry times
        .random_user_agent(true) // use random user agent from fake_user_agent database
        .wait(5); // wait time in seconds before solving challenge

    // With proxy (optional)
    // let mut bypasser = bypasser.proxy("http://127.0.0.1:1087");

    // Bypass Cloudflare protection
    println!("Attempting to bypass Cloudflare protection...");
    let (cookie, user_agent) = loop {
        match bypasser.bypass(WEBSITE).await {
            Ok((c, ua)) => {
                println!("Successfully bypassed Cloudflare!");
                break (c, ua);
            }
            Err(e) => {
                eprintln!("Failed to bypass: {}, retrying...", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }
    };

    // Use the obtained cookie and user-agent in subsequent requests
    println!("\nUsing obtained credentials to make a request...");

    // Method 1: Create a client with default headers
    {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::COOKIE, cookie.clone());
        headers.insert(reqwest::header::USER_AGENT, user_agent.clone());

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        let text = client.get(WEBSITE).send().await?.text().await?;
        println!("Response length: {} bytes", text.len());
    }

    // Method 2: Add headers per request
    {
        let client = reqwest::Client::new();
        let text = client
            .get(WEBSITE)
            .header(reqwest::header::COOKIE, cookie)
            .header(reqwest::header::USER_AGENT, user_agent)
            .send()
            .await?
            .text()
            .await?;
        println!("Response length (method 2): {} bytes", text.len());
    }

    Ok(())
}
