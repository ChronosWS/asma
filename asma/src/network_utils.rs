use std::net::IpAddr;

pub async fn refresh_ip() -> Result<IpAddr, ()> {
    let mut response = reqwest::get("https://api.ipify.org").await.map_err(|e| {
        eprintln!(
            "Error requesting IP from https://api.ipify.org: {}",
            e.to_string()
        )
    });

    if response.is_err() {
        response = reqwest::get("http://whatismyip.akamai.com")
            .await
            .map_err(|e| {
                eprintln!(
                    "Error requesting IP from http://whatismyip.akamai.com: {}",
                    e.to_string()
                )
            })
    }

    if let Ok(response) = response {
        if let Ok(text) = response
            .text()
            .await
            .map_err(|e| eprintln!("Failed to get response value: {}", e.to_string()))
        {
            return text.parse::<IpAddr>().map_err(|e| {
                eprintln!(
                    "Failed to parse IP address from response '{}': {}",
                    text,
                    e.to_string()
                )
            });
        }
    }
    Err(())
}