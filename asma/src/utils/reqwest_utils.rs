use std::sync::RwLock;

use futures_util::Future;
use reqwest::{Certificate, IntoUrl, Response};
use tracing::trace;

static CERTIFICATES: RwLock<Vec<Certificate>> = RwLock::new(vec![]);

pub fn init() {
    let ca_cert_text = include_str!("../../res/data/cacert.pem");

    enum Mode {
        Searching,
        Building(String),
    }
    let mut mode: Mode = Mode::Searching;
    let mut certs = vec![];
    for line in ca_cert_text.lines() {
        if line == "-----BEGIN CERTIFICATE-----\n" {
            if matches!(mode, Mode::Searching) {
                mode = Mode::Building(line.to_owned());
            }
        } else if line == "-----END CERTIFICATE-----\n" {
            if let Mode::Building(mut cert) = mode {
                cert.push_str(line);
                let cert = reqwest::tls::Certificate::from_pem(cert.as_bytes()).unwrap();
                certs.push(cert);
                trace!("Added cert");
                mode = Mode::Searching;
            }
        } else if let Mode::Building(mut cert) = mode {
            cert += line;
            mode = Mode::Building(cert);
        }
    }

    CERTIFICATES.write().unwrap().append(&mut certs);
}

pub fn client() -> reqwest::Client {
    let mut builder = reqwest::Client::builder().use_rustls_tls();
    for cert in CERTIFICATES.read().unwrap().iter() {
        builder = builder.add_root_certificate(cert.clone());
    }
    builder.build().unwrap()
}

pub fn get<U: IntoUrl>(url: U) -> impl Future<Output = Result<Response, reqwest::Error>> {
    client().get(url).send()
}
