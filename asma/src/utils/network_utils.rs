use std::net::IpAddr;

use crate::reqwest_utils;

pub async fn refresh_ip() -> Result<IpAddr, ()> {
    let mut response = reqwest_utils::get("https://api.ipify.org")
        .await
        .map_err(|e| eprintln!("Error requesting IP from https://api.ipify.org: {}", e));

    if response.is_err() {
        response = reqwest_utils::get("http://whatismyip.akamai.com")
            .await
            .map_err(|e| {
                eprintln!(
                    "Error requesting IP from http://whatismyip.akamai.com: {}",
                    e
                )
            })
    }

    if let Ok(response) = response {
        if let Ok(text) = response
            .text()
            .await
            .map_err(|e| eprintln!("Failed to get response value: {}", e))
        {
            return text.parse::<IpAddr>().map_err(|e| {
                eprintln!("Failed to parse IP address from response '{}': {}", text, e)
            });
        }
    }
    Err(())
}
