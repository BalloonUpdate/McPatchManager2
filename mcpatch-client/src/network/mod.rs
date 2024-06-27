pub mod http;
pub mod private;
pub mod webdav;

use std::ops::Range;
use std::pin::Pin;

use async_trait::async_trait;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;

use crate::error::BusinessError;
use crate::error::BusinessResult;
use crate::error::ResultToBusinessError;
use crate::global_config::GlobalConfig;
use crate::log::log_debug;
use crate::log::log_error;
use crate::log::log_info;
use crate::network::http::HttpProtocol;
use crate::network::private::PrivateProtocol;
use crate::network::webdav::Webdav;

pub type DownloadResult = std::io::Result<BusinessResult<(u64, Pin<Box<dyn AsyncRead + Send>>)>>;

pub struct Network<'a> {
    sources: Vec<Box<dyn UpdatingSource + Sync + Send>>,
    skip_sources: usize,
    config: &'a GlobalConfig,
}

impl<'a> Network<'a> {
    pub fn new(config: &'a GlobalConfig) -> BusinessResult<Self> {
        let mut sources = Vec::<Box<dyn UpdatingSource + Sync + Send>>::new();
        let mut index = 0u32;

        for url in &config.urls {
            if url.starts_with("http://") || url.starts_with("https://") {
                sources.push(Box::new(HttpProtocol::new(url, &config, index)))
            } else if url.starts_with("mcpatch://") {
                sources.push(Box::new(PrivateProtocol::new(&url["mcpatch://".len()..], &config, index)))
            } else if url.starts_with("webdav://") {
                sources.push(Box::new(Webdav::new(&url, &config, index)))
            } else {
                log_info(format!("unknown url: {}", url));
            }

            index += 1;
        }

        log_debug(format!("loaded {} urls", sources.len()));

        Ok(Network { sources, skip_sources: 0, config })
    }

    pub async fn request_text(&mut self, path: &str, range: Range<u64>, desc: impl AsRef<str>) -> BusinessResult<String> {
        match self.request_file(path, range, desc.as_ref()).await {
            Ok(ok) => {
                let (len, mut data) = ok;
                        
                let mut text = String::with_capacity(len as usize);
                data.read_to_string(&mut text).await.be(|e| format!("网络数据无法解码为utf8字符串({})，原因：{:?}", desc.as_ref(), e))?;
                Ok(text)
            },
            Err(err) => return Err(err),
        }
    }

    pub async fn request_file(&mut self, path: &str, range: Range<u64>, desc: &str) -> BusinessResult<(u64, Pin<Box<dyn AsyncRead + Send>>)> {
        assert!(range.end >= range.start);

        let mut io_error = Option::<std::io::Error>::None;
        let mut skip = 0;
        
        for source in &mut self.sources {
            if skip < self.skip_sources {
                skip += 1;
                continue;
            }

            log_debug(format!("+ request {} {}+{} ({})", path, range.start, range.end - range.start, desc));

            for i in 0..self.config.http_retries + 1 {
                match source.request(path, &range, desc.as_ref(), self.config).await {
                    Ok(ok) => {
                        match ok {
                            Ok(ok) => return Ok(ok),
                            Err(err) => return Err(err),
                        }
                    },
                    Err(err) => {
                        io_error = Some(err);
                        
                        if i != self.config.http_retries {
                            log_error("retrying")
                        }
                    },
                }
            }

            self.skip_sources += 1;
        }
        
        return Err(BusinessError::new(io_error.unwrap().to_string()));
    }

    pub fn advance_source(&mut self) {
        self.skip_sources += 1;
    }
}

#[async_trait]
pub trait UpdatingSource {
    async fn request(&mut self, path: &str, range: &Range<u64>, desc: &str, config: &GlobalConfig) -> DownloadResult;
}
