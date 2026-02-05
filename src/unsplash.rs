use serde::Deserialize;
use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Deserialize)]
pub struct UnsplashPhoto {
    pub id: String,
    pub description: Option<String>,
    pub alt_description: Option<String>,
    pub urls: UnsplashUrls,
    pub user: UnsplashUser,
    pub links: UnsplashLinks,
}

#[derive(Debug, Deserialize)]
pub struct UnsplashUrls {
    pub full: String,
}

#[derive(Debug, Deserialize)]
pub struct UnsplashUser {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct UnsplashLinks {
    pub html: String,
}

pub struct UnsplashClient {
    client: reqwest::Client,
    access_key: String,
}

impl UnsplashClient {
    pub fn new(access_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            access_key,
        }
    }

    pub async fn fetch_random(&self, collections: &[String]) -> Result<UnsplashPhoto> {
        let url = "https://api.unsplash.com/photos/random";
        let collection_str = collections.join(",");

        let response = self.client
            .get(url)
            .header("Authorization", format!("Client-ID {}", self.access_key))
            .query(&[
                ("collections", collection_str.as_str()),
                ("orientation", "landscape"),
                ("count", "1"),
            ])
            .send()
            .await
            .context("Failed to send Unsplash request")?;

        let status = response.status();
        if !status.is_success() {
             let text = response.text().await.unwrap_or_default();
             anyhow::bail!("Unsplash API Error {}: {}", status, text);
        }

        let photos: Vec<UnsplashPhoto> = response.json().await
            .context("Failed to parse Unsplash response")?;

        photos.into_iter().next().context("No photos returned")
    }

    pub async fn download_image(&self, url: &str, path: &PathBuf) -> Result<()> {
        let response = self.client.get(url).send().await
            .context("Failed to download image")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to download image: status {}", response.status());
        }

        let bytes = response.bytes().await
            .context("Failed to get image bytes")?;
        
        // Ensure directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await
                .context("Failed to create wallpaper directory")?;
        }

        let mut file = tokio::fs::File::create(path).await
            .context("Failed to create image file")?;
            
        file.write_all(&bytes).await
            .context("Failed to write image to file")?;

        Ok(())
    }
}
