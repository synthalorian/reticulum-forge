use crate::deploy::inventory::Node;
use crate::error::{ForgeError, ForgeResult};
use async_trait::async_trait;
use russh::client;
use russh::keys::{self, key};
use std::sync::Arc;
use std::time::Duration;

/// Configuration for an SSH connection derived from a `Node`.
#[derive(Debug, Clone)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub key_path: String,
    pub key_passphrase: Option<String>,
    pub timeout_secs: u64,
    pub verify_host_key: bool,
}

impl Default for SshConfig {
    fn default() -> Self {
        SshConfig {
            host: String::new(),
            port: 22,
            user: "root".into(),
            key_path: "~/.ssh/id_ed25519".into(),
            key_passphrase: None,
            timeout_secs: 30,
            verify_host_key: false,
        }
    }
}

impl From<&Node> for SshConfig {
    fn from(node: &Node) -> Self {
        SshConfig {
            host: node.host.clone(),
            port: node.port,
            user: node.user.clone(),
            key_path: node.key_path.clone(),
            key_passphrase: node.key_passphrase.clone(),
            timeout_secs: 30,
            verify_host_key: false,
        }
    }
}

/// Result of a single SSH command execution.
#[derive(Debug)]
pub struct SshResult {
    pub exit_code: u32,
    pub stdout: String,
    pub stderr: String,
}

impl SshResult {
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// SSH client wrapping a russh session handle.
pub struct SshClient {
    handle: client::Handle<SshHandler>,
    config: SshConfig,
}

struct SshHandler {
    verify_host_key: bool,
}

impl SshHandler {
    fn new(verify: bool) -> Self {
        Self {
            verify_host_key: verify,
        }
    }
}

#[async_trait]
impl client::Handler for SshHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        if self.verify_host_key {
            tracing::warn!(
                "Host key verification requested but known_hosts lookup not yet implemented. \
                 Fingerprint: {}",
                server_public_key.fingerprint()
            );
            Ok(true)
        } else {
            tracing::debug!("TOFU: accepting server host key");
            Ok(true)
        }
    }
}

impl SshClient {
    /// Connect to a remote host and authenticate with public key.
    pub async fn connect(config: &SshConfig) -> ForgeResult<Self> {
        let expanded_key_path = shellexpand::tilde(&config.key_path).to_string();

        let key_pair = keys::load_secret_key(&expanded_key_path, config.key_passphrase.as_deref())
            .map_err(|e| {
                ForgeError::Ssh(format!(
                    "failed to load SSH key '{}': {}",
                    expanded_key_path, e
                ))
            })?;

        let russh_config = client::Config {
            inactivity_timeout: Some(Duration::from_secs(config.timeout_secs)),
            ..<_>::default()
        };

        let config_arc = Arc::new(russh_config);
        let handler = SshHandler::new(config.verify_host_key);

        let mut handle = client::connect(config_arc, (&config.host[..], config.port), handler)
            .await
            .map_err(|e| {
                ForgeError::Ssh(format!(
                    "connection failed ({}:{}): {}",
                    config.host, config.port, e
                ))
            })?;

        let auth_res = handle
            .authenticate_publickey(&config.user, Arc::new(key_pair))
            .await
            .map_err(|e| {
                ForgeError::Ssh(format!(
                    "authentication failed ({}@{}): {}",
                    config.user, config.host, e
                ))
            })?;

        if !auth_res {
            return Err(ForgeError::Ssh(format!(
                "public key authentication rejected for {}@{}",
                config.user, config.host
            )));
        }

        tracing::debug!(
            "SSH connected and authenticated: {}@{}:{}",
            config.user,
            config.host,
            config.port
        );

        Ok(SshClient {
            handle,
            config: config.clone(),
        })
    }

    /// Execute a shell command and capture output.
    pub async fn execute(&mut self, command: &str) -> ForgeResult<SshResult> {
        let mut channel = self
            .handle
            .channel_open_session()
            .await
            .map_err(|e| ForgeError::Ssh(format!("channel open failed: {}", e)))?;

        channel
            .exec(true, command)
            .await
            .map_err(|e| ForgeError::Ssh(format!("exec failed: {}", e)))?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut exit_code: Option<u32> = None;

        loop {
            let Some(msg) = channel.wait().await else {
                break;
            };
            match msg {
                russh::ChannelMsg::Data { ref data } => {
                    stdout.extend_from_slice(data);
                }
                russh::ChannelMsg::ExtendedData { ref data, ext: 1 } => {
                    stderr.extend_from_slice(data);
                }
                russh::ChannelMsg::ExitStatus { exit_status } => {
                    exit_code = Some(exit_status);
                    // russh may send Eof/Close after ExitStatus — keep reading
                    // until the channel actually closes to capture all output.
                }
                russh::ChannelMsg::Eof | russh::ChannelMsg::Close
                    // Only break if we've already seen the exit status.
                    // Some russh versions send Close before ExitStatus.
                    if exit_code.is_some() => {
                        break;
                    }
                _ => {}
            }
        }

        // If we never got an ExitStatus but the channel closed cleanly
        // and there's no stderr, treat as success (exit 0).
        let code = exit_code.unwrap_or({
            if stderr.is_empty() { 0 } else { 255 }
        });
        Ok(SshResult {
            exit_code: code,
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
        })
    }

    /// Transfer a text file via heredoc.
    pub async fn write_file(&mut self, remote_path: &str, content: &str) -> ForgeResult<()> {
        if remote_path.contains("..") {
            return Err(ForgeError::Validation(format!(
                "remote path must not contain '..' (directory traversal): {}",
                remote_path
            )));
        }
        if remote_path.contains(';')
            || remote_path.contains('|')
            || remote_path.contains('$')
            || remote_path.contains('`')
        {
            return Err(ForgeError::Validation(format!(
                "remote path must not contain shell metacharacters: {}",
                remote_path
            )));
        }

        if let Some(parent) = std::path::Path::new(remote_path).parent() {
            let mkdir_cmd = format!("mkdir -p {}", parent.display());
            let mkdir_res = self.execute(&mkdir_cmd).await?;
            if !mkdir_res.success() {
                return Err(ForgeError::Ssh(format!(
                    "failed to create remote directory '{}': {}",
                    parent.display(),
                    mkdir_res.stderr.trim()
                )));
            }
        }

        // Security: use printf with single-quoted content to avoid heredoc
        // delimiter collision and shell injection. Single quotes inside the
        // content are escaped by ending the quote, inserting a literal quote,
        // and starting a new quote.
        let escaped = content.replace('\'', "'\\''");
        let write_cmd = format!(
            "printf '%s' '{}' > {} && chmod 644 {}",
            escaped, remote_path, remote_path
        );
        let res = self.execute(&write_cmd).await?;
        if !res.success() {
            return Err(ForgeError::Ssh(format!(
                "failed to write remote file '{}': {}",
                remote_path,
                res.stderr.trim()
            )));
        }

        tracing::debug!("Wrote {} bytes to remote '{}'", content.len(), remote_path);
        Ok(())
    }

    /// Read a remote file's contents.
    pub async fn read_file(&mut self, remote_path: &str) -> ForgeResult<String> {
        if remote_path.contains("..") {
            return Err(ForgeError::Validation(format!(
                "remote path must not contain '..' (directory traversal): {}",
                remote_path
            )));
        }

        let cmd = format!("cat {}", remote_path);
        let res = self.execute(&cmd).await?;
        if !res.success() {
            return Err(ForgeError::Ssh(format!(
                "failed to read remote file '{}': {}",
                remote_path,
                res.stderr.trim()
            )));
        }
        Ok(res.stdout)
    }

    /// Check if a file exists on the remote host.
    #[allow(dead_code)]
    pub async fn file_exists(&mut self, remote_path: &str) -> ForgeResult<bool> {
        let cmd = format!("test -f {}", remote_path);
        let res = self.execute(&cmd).await?;
        Ok(res.success())
    }

    /// Check if a directory exists on the remote host.
    #[allow(dead_code)]
    pub async fn dir_exists(&mut self, remote_path: &str) -> ForgeResult<bool> {
        let cmd = format!("test -d {}", remote_path);
        let res = self.execute(&cmd).await?;
        Ok(res.success())
    }

    /// Close the SSH connection cleanly.
    pub async fn close(self) -> ForgeResult<()> {
        self.handle
            .disconnect(russh::Disconnect::ByApplication, "", "English")
            .await
            .map_err(|e| ForgeError::Ssh(format!("disconnect error: {}", e)))?;
        tracing::debug!(
            "SSH connection closed: {}@{}",
            self.config.user,
            self.config.host
        );
        Ok(())
    }

    #[allow(dead_code)]
    pub fn config(&self) -> &SshConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_config_from_node() {
        let node = Node {
            host: "10.0.0.1".into(),
            port: 2222,
            user: "admin".into(),
            key_path: "/etc/forge/key".into(),
            key_passphrase: None,
            tags: vec![],
            rns_install_path: "/opt/reticulum".into(),
            config_path: "/etc/reticulum/config.conf".into(),
            service_name: "rnsd".into(),
        };
        let config = SshConfig::from(&node);
        assert_eq!(config.host, "10.0.0.1");
        assert_eq!(config.port, 2222);
        assert_eq!(config.user, "admin");
        assert_eq!(config.key_path, "/etc/forge/key");
        assert_eq!(config.timeout_secs, 30);
    }

    #[test]
    fn test_result_success() {
        let r = SshResult {
            exit_code: 0,
            stdout: "ok".into(),
            stderr: String::new(),
        };
        assert!(r.success());

        let r = SshResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".into(),
        };
        assert!(!r.success());
    }
}
