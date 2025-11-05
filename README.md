## Intro

**cloudflare-bypasser** (Modernized Fork)

This is a modernized fork of the original [cloudflare-bypasser](https://github.com/AurevoirXavier/cloudflare-bypasser) by Xavier Lau.

**Key improvements in this fork:**
- **Fully async/await**: Migrated from `reqwest::blocking` to async `reqwest::Client`
- **Rustls instead of native-tls**: More secure and cross-platform friendly
- **Modern dependencies**: Updated to reqwest 0.12, tokio 1.41, and latest crates
- **No blocking runtime**: No internal tokio runtime, consumer controls the executor
- **Real user-agent database**: Uses `fake_user_agent` with hundreds of real, up-to-date browser UAs
- **Better error handling**: Uses `eprintln!` for better error reporting

Inspired by the Python module [cloudflare-scrape](https://github.com/Anorov/cloudflare-scrape)

## Requirements

- **Node.js** (required for JavaScript challenge execution)
- **Rust 1.70+** (for `async fn` in traits)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
cloudflare-bypasser = { git = "https://github.com/Ryujin-K/cloudflare-bypasser-rs" }
```

## Example

```rust
use cloudflare_bypasser::Bypasser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    const WEBSITE: &str = "https://example.com";

    // Quick start
    let mut bypasser = Bypasser::default();

    // Or customize
    let mut bypasser = Bypasser::default()
        .retry(30)                      // retry times, default 1
        .random_user_agent(true)        // use random user agent from fake_user_agent database
        .wait(5);                       // cloudflare's waiting time in seconds, default 3

    // Optional: use proxy
    // let mut bypasser = bypasser.proxy("http://127.0.0.1:1087");

    // Bypass and obtain cookie + user-agent
    let (cookie, user_agent) = loop {
        if let Ok((c, ua)) = bypasser.bypass(WEBSITE).await {
            break (c, ua);
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    };
    
    // Method 1: Use reqwest with default headers
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(reqwest::header::COOKIE, cookie.clone());
    headers.insert(reqwest::header::USER_AGENT, user_agent.clone());
    
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;
        
    let text = client.get(WEBSITE).send().await?.text().await?;
    println!("{}", text);
    
    // Method 2: Per-request headers
    let text = reqwest::Client::new()
        .get(WEBSITE)
        .header(reqwest::header::COOKIE, cookie)
        .header(reqwest::header::USER_AGENT, user_agent)
        .send()
        .await?
        .text()
        .await?;
    println!("{}", text);
    
    Ok(())
}
```

## Migration from Original Version

If you're migrating from the original blocking version:

1. **Add tokio runtime**: Wrap your `main()` with `#[tokio::main]`
2. **Make calls async**: Change `bypasser.bypass(url)` to `bypasser.bypass(url).await`
3. **Update Cargo.toml**: Use the new repository URL
4. **Remove blocking client**: Replace `reqwest::blocking::Client` with async `reqwest::Client`

## Running the Example

```bash
cargo run --example async_bypass
```

## License

MIT/Apache-2.0 (same as original project)

## Credits

- Original author: [Xavier Lau](https://github.com/AurevoirXavier)
- Modernization: [Ryujin-K](https://github.com/Ryujin-K)
