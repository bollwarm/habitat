use pin_project::pin_project;
use rustls::{ClientConfig as TlsClientConfig,
             ServerConfig as TlsServerConfig};
use std::{pin::Pin,
          sync::Arc,
          task::{Context,
                 Poll}};
use tokio::{io::{self,
                 AsyncRead,
                 AsyncWrite},
            net::TcpStream};
use tokio_rustls::{webpki::DNSNameRef,
                   TlsAcceptor,
                   TlsConnector,
                   TlsStream};

/// A wrapper type that can either be a raw TCP stream or a TCP stream with TLS.
#[pin_project(project = TcpOrTlsStreamProj)]
pub enum TcpOrTlsStream {
    TcpStream(#[pin] TcpStream),
    TlsStream(#[pin] TlsStream<TcpStream>),
}

impl TcpOrTlsStream {
    /// Create a new `TcpStream`
    pub fn new(stream: TcpStream) -> Self { Self::TcpStream(stream) }

    /// Create a new `TlsStream` using server configuration
    pub async fn new_tls_server(stream: TcpStream,
                                tls_config: Arc<TlsServerConfig>)
                                -> io::Result<Self> {
        let tcp_stream = Self::new(stream);
        tcp_stream.to_tls_server(tls_config).await
    }

    /// Create a new `TlsStream` using client configuration
    pub async fn new_tls_client(stream: TcpStream,
                                tls_config: Arc<TlsClientConfig>,
                                domain: &str)
                                -> io::Result<Self> {
        let tcp_stream = Self::new(stream);
        tcp_stream.to_tls_client(tls_config, domain).await
    }

    /// Upgrade a `TcpStream` into a `TlsStream` using server configuration
    pub async fn to_tls_server(self, tls_config: Arc<TlsServerConfig>) -> io::Result<Self> {
        Ok(match self {
            Self::TcpStream(stream) => {
                let tls_connector = TlsAcceptor::from(tls_config);
                let tls_stream = tls_connector.accept(stream)
                                              .into_failable()
                                              .await
                                              .map_err(|e| e.0)?;
                Self::TlsStream(TlsStream::Server(tls_stream))
            }
            Self::TlsStream(stream) => Self::TlsStream(stream),
        })
    }

    /// Upgrade a `TcpStream` to a `TlsStream` using client configuration
    pub async fn to_tls_client(self,
                               tls_config: Arc<TlsClientConfig>,
                               domain: &str)
                               -> io::Result<Self> {
        Ok(match self {
            Self::TcpStream(stream) => {
                let tls_connector = TlsConnector::from(tls_config);
                let domain = DNSNameRef::try_from_ascii_str(domain).map_err(|_| {
                                 io::Error::new(io::ErrorKind::InvalidInput,
                                                format!("invalid DNS name '{}'", domain))
                             })?;
                let tls_stream = tls_connector.connect(domain, stream)
                                              .into_failable()
                                              .await
                                              .map_err(|e| e.0)?;
                Self::TlsStream(TlsStream::Client(tls_stream))
            }
            Self::TlsStream(stream) => Self::TlsStream(stream),
        })
    }
}

impl AsyncRead for TcpOrTlsStream {
    fn poll_read(self: Pin<&mut Self>,
                 cx: &mut Context,
                 buf: &mut [u8])
                 -> Poll<io::Result<usize>> {
        match self.project() {
            TcpOrTlsStreamProj::TcpStream(stream) => stream.poll_read(cx, buf),
            TcpOrTlsStreamProj::TlsStream(stream) => stream.poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for TcpOrTlsStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        match self.project() {
            TcpOrTlsStreamProj::TcpStream(stream) => stream.poll_write(cx, buf),
            TcpOrTlsStreamProj::TlsStream(stream) => stream.poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        match self.project() {
            TcpOrTlsStreamProj::TcpStream(stream) => stream.poll_flush(cx),
            TcpOrTlsStreamProj::TlsStream(stream) => stream.poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        match self.project() {
            TcpOrTlsStreamProj::TcpStream(stream) => stream.poll_shutdown(cx),
            TcpOrTlsStreamProj::TlsStream(stream) => stream.poll_shutdown(cx),
        }
    }
}