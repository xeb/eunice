use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::Serialize;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::common::{error_content, text_content, McpServer, McpTool};

/// Configuration for the browser server
pub struct BrowserServerConfig {
    /// Port for Chrome DevTools Protocol
    pub port: u16,
    /// User data directory for Chrome profile
    pub user_data_dir: PathBuf,
    /// Path to Chrome executable
    pub chrome_path: String,
    /// Page load timeout in seconds
    pub timeout: Duration,
    /// Whether running in HTTP mode
    pub http_mode: bool,
    /// Enable verbose logging
    pub verbose: bool,
}

impl BrowserServerConfig {
    pub fn new(port: u16, chrome_path: Option<String>, timeout: u64, verbose: bool) -> Self {
        Self::with_user_data_dir(port, chrome_path, None, timeout, verbose)
    }

    pub fn with_user_data_dir(
        port: u16,
        chrome_path: Option<String>,
        user_data_dir: Option<PathBuf>,
        timeout: u64,
        verbose: bool,
    ) -> Self {
        let user_data_dir =
            user_data_dir.unwrap_or_else(|| PathBuf::from(format!("/tmp/mcpz/browser-{}", port)));

        // Find Chrome path - use provided path or search common locations
        let chrome_path = chrome_path.unwrap_or_else(Self::find_chrome);

        Self {
            port,
            user_data_dir,
            chrome_path,
            timeout: Duration::from_secs(timeout),
            http_mode: false,
            verbose,
        }
    }

    /// Find Chrome executable by searching common installation paths
    fn find_chrome() -> String {
        let candidates: Vec<&str> = if cfg!(target_os = "macos") {
            vec![
                "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
                "/Applications/Chromium.app/Contents/MacOS/Chromium",
                "/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary",
                "/usr/local/bin/google-chrome",
                "/usr/local/bin/chromium",
            ]
        } else if cfg!(target_os = "windows") {
            vec![
                r"C:\Program Files\Google\Chrome\Application\chrome.exe",
                r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
                r"C:\Users\%USERNAME%\AppData\Local\Google\Chrome\Application\chrome.exe",
                r"C:\Program Files\Chromium\Application\chrome.exe",
            ]
        } else {
            // Linux - check multiple common paths
            vec![
                "/usr/bin/google-chrome",
                "/usr/bin/google-chrome-stable",
                "/usr/bin/chromium",
                "/usr/bin/chromium-browser",
                "/snap/bin/chromium",
                "/usr/local/bin/google-chrome",
                "/usr/local/bin/chromium",
                "/opt/google/chrome/google-chrome",
                "/opt/google/chrome/chrome",
                "/opt/chromium/chromium",
            ]
        };

        // Return first existing path, or fallback to a reasonable default
        for candidate in &candidates {
            if std::path::Path::new(candidate).exists() {
                return candidate.to_string();
            }
        }

        // Fallback: try PATH lookup via `which` equivalent
        if cfg!(target_os = "macos") {
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome".to_string()
        } else if cfg!(target_os = "windows") {
            r"C:\Program Files\Google\Chrome\Application\chrome.exe".to_string()
        } else {
            // Linux: try common names that might be in PATH
            "google-chrome".to_string()
        }
    }

    /// Set HTTP mode (screenshots return base64 instead of file path)
    pub fn with_http_mode(mut self, http_mode: bool) -> Self {
        self.http_mode = http_mode;
        self
    }
}

/// Browser automation result
#[derive(Serialize)]
pub struct BrowserResult {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Screenshot result
pub struct ScreenshotResult {
    /// File path where screenshot was saved
    pub path: PathBuf,
    /// Whether full page was captured
    pub full_page: bool,
}

/// Chrome DevTools Protocol client
struct ChromeClient {
    stream: TcpStream,
    message_id: u32,
}

impl ChromeClient {
    fn connect(port: u16, timeout: Duration) -> Result<Self> {
        Self::connect_to_tab(port, timeout, None)
    }

    fn connect_to_tab_by_id(port: u16, timeout: Duration, tab_id: &str) -> Result<Self> {
        // Get the WebSocket URL from Chrome's HTTP endpoint
        let http_url = format!("http://localhost:{}/json", port);

        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()?;

        let response = client
            .get(&http_url)
            .send()
            .context("Failed to connect to Chrome DevTools")?;

        let tabs: Vec<serde_json::Value> = response.json().context("Failed to parse Chrome tabs")?;

        // Find tab by ID
        let tab = tabs
            .iter()
            .find(|t| t.get("id").and_then(|v| v.as_str()) == Some(tab_id))
            .ok_or_else(|| anyhow::anyhow!("Tab not found: {}", tab_id))?;

        let ws_url = tab
            .get("webSocketDebuggerUrl")
            .and_then(|url| url.as_str())
            .ok_or_else(|| anyhow::anyhow!("No WebSocket URL found for tab"))?;

        Self::connect_to_ws_url(ws_url, timeout)
    }

    fn connect_to_tab(port: u16, timeout: Duration, target_url: Option<&str>) -> Result<Self> {
        // First, get the WebSocket URL from Chrome's HTTP endpoint
        let http_url = format!("http://localhost:{}/json", port);

        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()?;

        let response = client
            .get(&http_url)
            .send()
            .context("Failed to connect to Chrome DevTools")?;

        let tabs: Vec<serde_json::Value> = response.json().context("Failed to parse Chrome tabs")?;

        // If we have a target URL, try to find that tab first
        let tab = if let Some(target) = target_url {
            tabs.iter().find(|t| {
                t.get("url").and_then(|v| v.as_str()).map(|u| u.contains(target)).unwrap_or(false)
            })
        } else {
            None
        };

        // If no target URL match, find a "page" type tab (not extension background pages or offscreen documents)
        // Prefer tabs with actual URLs over chrome:// or extension pages
        let tab = tab.or_else(|| {
            tabs.iter().find(|t| {
                let tab_type = t.get("type").and_then(|v| v.as_str()).unwrap_or("");
                let url = t.get("url").and_then(|v| v.as_str()).unwrap_or("");
                tab_type == "page"
                    && !url.starts_with("chrome://")
                    && !url.starts_with("chrome-extension://")
                    && !url.contains("offscreen")
            })
        }).or_else(|| {
            // Fall back to any "page" type tab
            tabs.iter().find(|t| {
                t.get("type").and_then(|v| v.as_str()) == Some("page")
            })
        }).or_else(|| tabs.first()); // Last resort: first tab

        let ws_url = tab
            .and_then(|tab| tab.get("webSocketDebuggerUrl"))
            .and_then(|url| url.as_str())
            .ok_or_else(|| anyhow::anyhow!("No WebSocket URL found"))?;

        Self::connect_to_ws_url(ws_url, timeout)
    }

    fn connect_to_ws_url(ws_url: &str, timeout: Duration) -> Result<Self> {

        // Parse WebSocket URL to get host and path
        let ws_url = ws_url.trim_start_matches("ws://");
        let parts: Vec<&str> = ws_url.splitn(2, '/').collect();
        let host_port = parts[0];
        let path = format!("/{}", parts.get(1).unwrap_or(&""));

        // Connect via TCP
        let stream =
            TcpStream::connect(host_port).context("Failed to connect to Chrome WebSocket")?;
        stream.set_read_timeout(Some(timeout))?;
        stream.set_write_timeout(Some(timeout))?;

        let mut client = Self {
            stream,
            message_id: 0,
        };

        // Perform WebSocket handshake
        client.websocket_handshake(&host_port.to_string(), &path)?;

        Ok(client)
    }

    fn websocket_handshake(&mut self, host: &str, path: &str) -> Result<()> {
        let key = BASE64.encode(rand::random::<[u8; 16]>());

        let request = format!(
            "GET {} HTTP/1.1\r\n\
             Host: {}\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Key: {}\r\n\
             Sec-WebSocket-Version: 13\r\n\
             \r\n",
            path, host, key
        );

        self.stream.write_all(request.as_bytes())?;

        // Read response
        let mut reader = BufReader::new(&self.stream);
        let mut response = String::new();

        loop {
            let mut line = String::new();
            reader.read_line(&mut line)?;
            if line == "\r\n" {
                break;
            }
            response.push_str(&line);
        }

        if !response.contains("101") {
            return Err(anyhow::anyhow!(
                "WebSocket handshake failed: {}",
                response
            ));
        }

        Ok(())
    }

    fn send_command(&mut self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        self.message_id += 1;
        let message = serde_json::json!({
            "id": self.message_id,
            "method": method,
            "params": params
        });

        let json = serde_json::to_string(&message)?;
        self.send_ws_message(&json)?;

        // Read response
        loop {
            let response = self.read_ws_message()?;
            let parsed: serde_json::Value = serde_json::from_str(&response)?;

            // Check if this is our response
            if parsed.get("id") == Some(&serde_json::json!(self.message_id)) {
                if let Some(error) = parsed.get("error") {
                    return Err(anyhow::anyhow!("CDP error: {}", error));
                }
                return Ok(parsed.get("result").cloned().unwrap_or(serde_json::json!({})));
            }
        }
    }

    fn send_ws_message(&mut self, message: &str) -> Result<()> {
        let payload = message.as_bytes();
        let len = payload.len();

        // Build WebSocket frame (text frame, masked)
        let mut frame = Vec::new();

        // Opcode 0x81 = final frame + text
        frame.push(0x81);

        // Length with mask bit set
        if len < 126 {
            frame.push((len as u8) | 0x80);
        } else if len < 65536 {
            frame.push(126 | 0x80);
            frame.push((len >> 8) as u8);
            frame.push((len & 0xff) as u8);
        } else {
            frame.push(127 | 0x80);
            for i in (0..8).rev() {
                frame.push((len >> (i * 8)) as u8);
            }
        }

        // Masking key
        let mask: [u8; 4] = rand::random();
        frame.extend_from_slice(&mask);

        // Masked payload
        for (i, byte) in payload.iter().enumerate() {
            frame.push(byte ^ mask[i % 4]);
        }

        self.stream.write_all(&frame)?;
        self.stream.flush()?;

        Ok(())
    }

    fn read_ws_message(&mut self) -> Result<String> {
        let mut header = [0u8; 2];
        std::io::Read::read_exact(&mut self.stream, &mut header)?;

        let _opcode = header[0] & 0x0f;
        let masked = (header[1] & 0x80) != 0;
        let mut len = (header[1] & 0x7f) as usize;

        if len == 126 {
            let mut extended = [0u8; 2];
            std::io::Read::read_exact(&mut self.stream, &mut extended)?;
            len = ((extended[0] as usize) << 8) | (extended[1] as usize);
        } else if len == 127 {
            let mut extended = [0u8; 8];
            std::io::Read::read_exact(&mut self.stream, &mut extended)?;
            len = 0;
            for byte in extended.iter() {
                len = (len << 8) | (*byte as usize);
            }
        }

        let mask = if masked {
            let mut m = [0u8; 4];
            std::io::Read::read_exact(&mut self.stream, &mut m)?;
            Some(m)
        } else {
            None
        };

        let mut payload = vec![0u8; len];
        std::io::Read::read_exact(&mut self.stream, &mut payload)?;

        if let Some(m) = mask {
            for (i, byte) in payload.iter_mut().enumerate() {
                *byte ^= m[i % 4];
            }
        }

        String::from_utf8(payload).context("Invalid UTF-8 in WebSocket message")
    }
}

/// Browser MCP server
pub struct BrowserServer {
    config: BrowserServerConfig,
    chrome_process: Arc<Mutex<Option<Child>>>,
    current_url: Arc<Mutex<Option<String>>>,
}

impl BrowserServer {
    pub fn new(config: BrowserServerConfig) -> Self {
        Self {
            config,
            chrome_process: Arc::new(Mutex::new(None)),
            current_url: Arc::new(Mutex::new(None)),
        }
    }

    /// Get a client connected to the current tab
    fn get_client(&self) -> Result<ChromeClient> {
        let url = self.current_url.lock().unwrap().clone();
        ChromeClient::connect_to_tab(self.config.port, self.config.timeout, url.as_deref())
            .context("Browser not running. Call start_browser first.")
    }

    pub fn start_browser(&self, headless: bool) -> Result<BrowserResult> {
        let mut process_guard = self.chrome_process.lock().unwrap();

        // Check if already running
        if let Some(ref mut child) = *process_guard {
            match child.try_wait() {
                Ok(None) => {
                    return Ok(BrowserResult {
                        success: true,
                        message: format!("Browser already running on port {}", self.config.port),
                        data: Some(serde_json::json!({
                            "port": self.config.port,
                            "user_data_dir": self.config.user_data_dir.display().to_string()
                        })),
                    });
                }
                _ => {}
            }
        }

        // Create user data directory
        std::fs::create_dir_all(&self.config.user_data_dir)?;

        self.log(&format!(
            "Starting Chrome on port {} with profile at {} (headless: {})",
            self.config.port,
            self.config.user_data_dir.display(),
            headless
        ));

        let mut cmd = Command::new(&self.config.chrome_path);
        cmd.arg(format!("--remote-debugging-port={}", self.config.port))
            .arg(format!(
                "--user-data-dir={}",
                self.config.user_data_dir.display()
            ))
            .arg(format!(
                "--remote-allow-origins=http://localhost:{}",
                self.config.port
            ))
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--window-size=1920,1080")
            .arg("--window-position=0,0");

        if headless {
            // Use new headless mode (Chrome 112+)
            cmd.arg("--headless=new");
            cmd.arg("--disable-gpu");
        }

        let child = cmd
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to start Chrome")?;

        *process_guard = Some(child);

        // Wait for Chrome to be ready
        std::thread::sleep(Duration::from_secs(2));

        // Verify connection and set viewport
        for attempt in 0..10 {
            match ChromeClient::connect(self.config.port, Duration::from_secs(5)) {
                Ok(mut client) => {
                    // Set viewport size via Emulation domain
                    let _ = client.send_command("Emulation.setDeviceMetricsOverride", serde_json::json!({
                        "width": 1920,
                        "height": 1080,
                        "deviceScaleFactor": 1,
                        "mobile": false
                    }));

                    return Ok(BrowserResult {
                        success: true,
                        message: format!(
                            "Browser started successfully on port {}",
                            self.config.port
                        ),
                        data: Some(serde_json::json!({
                            "port": self.config.port,
                            "user_data_dir": self.config.user_data_dir.display().to_string()
                        })),
                    });
                }
                Err(_) if attempt < 9 => {
                    std::thread::sleep(Duration::from_millis(500));
                }
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "Chrome started but DevTools not accessible: {}",
                        e
                    ));
                }
            }
        }

        Ok(BrowserResult {
            success: true,
            message: format!("Browser started on port {}", self.config.port),
            data: Some(serde_json::json!({
                "port": self.config.port,
                "user_data_dir": self.config.user_data_dir.display().to_string()
            })),
        })
    }

    pub fn open_url(&self, url: &str, wait_time: u64, tab_id: Option<&str>) -> Result<BrowserResult> {
        let mut client = if let Some(id) = tab_id {
            ChromeClient::connect_to_tab_by_id(self.config.port, self.config.timeout, id)
                .context("Failed to connect to specified tab")?
        } else {
            self.get_client()?
        };

        self.log(&format!("Navigating to: {}", url));

        // Enable Page domain
        client.send_command("Page.enable", serde_json::json!({}))?;

        // Navigate to URL
        client.send_command("Page.navigate", serde_json::json!({ "url": url }))?;

        // Store the URL for future connections (only if not targeting specific tab)
        if tab_id.is_none() {
            *self.current_url.lock().unwrap() = Some(url.to_string());
        }

        // Wait for page to load
        std::thread::sleep(Duration::from_secs(wait_time));

        Ok(BrowserResult {
            success: true,
            message: format!("Navigated to {}", url),
            data: Some(serde_json::json!({
                "url": url,
                "tab_id": tab_id
            })),
        })
    }

    pub fn get_page(&self) -> Result<BrowserResult> {
        let mut client = self.get_client()?;

        // Enable DOM
        client.send_command("DOM.enable", serde_json::json!({}))?;

        // Get document
        let doc = client.send_command("DOM.getDocument", serde_json::json!({ "depth": -1 }))?;

        let node_id = doc
            .get("root")
            .and_then(|r| r.get("nodeId"))
            .ok_or_else(|| anyhow::anyhow!("Could not get document node"))?;

        // Get outer HTML
        let html = client.send_command("DOM.getOuterHTML", serde_json::json!({ "nodeId": node_id }))?;

        let outer_html = html
            .get("outerHTML")
            .and_then(|h| h.as_str())
            .unwrap_or("")
            .to_string();

        Ok(BrowserResult {
            success: true,
            message: "Page content retrieved".to_string(),
            data: Some(serde_json::json!({ "html": outer_html })),
        })
    }

    pub fn get_page_as_markdown(&self) -> Result<BrowserResult> {
        let page_result = self.get_page()?;

        let html = page_result
            .data
            .as_ref()
            .and_then(|d| d.get("html"))
            .and_then(|h| h.as_str())
            .unwrap_or("");

        // Convert HTML to markdown using a simple conversion
        let markdown = html_to_markdown(html);

        Ok(BrowserResult {
            success: true,
            message: "Page content converted to markdown".to_string(),
            data: Some(serde_json::json!({ "markdown": markdown })),
        })
    }

    pub fn save_page_contents(&self, filepath: &str) -> Result<BrowserResult> {
        let mut client = self.get_client()?;

        // Enable DOM
        client.send_command("DOM.enable", serde_json::json!({}))?;

        // Get document with full depth
        let doc = client.send_command("DOM.getDocument", serde_json::json!({ "depth": -1 }))?;

        let node_id = doc
            .get("root")
            .and_then(|r| r.get("nodeId"))
            .ok_or_else(|| anyhow::anyhow!("Could not get document node"))?;

        // Get outer HTML of the entire document
        let html = client.send_command("DOM.getOuterHTML", serde_json::json!({ "nodeId": node_id }))?;

        let outer_html = html
            .get("outerHTML")
            .and_then(|h| h.as_str())
            .unwrap_or("")
            .to_string();

        let path = std::path::PathBuf::from(filepath);

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .context("Failed to create parent directory")?;
            }
        }

        // Write HTML to file
        std::fs::write(&path, &outer_html)
            .context("Failed to write HTML file")?;

        self.log(&format!("Page contents saved to: {}", path.display()));

        Ok(BrowserResult {
            success: true,
            message: format!("Page contents saved to {}", path.display()),
            data: Some(serde_json::json!({
                "path": path.display().to_string(),
                "size_bytes": outer_html.len()
            })),
        })
    }

    pub fn get_screenshot(&self, filepath: &str, full_page: bool) -> Result<ScreenshotResult> {
        let mut client = self.get_client()?;

        // Enable Page domain
        client.send_command("Page.enable", serde_json::json!({}))?;

        let mut params = serde_json::json!({
            "format": "png",
            "fromSurface": true
        });

        if full_page {
            // Get layout metrics for full page screenshot
            let metrics = client.send_command("Page.getLayoutMetrics", serde_json::json!({}))?;

            if let Some(content_size) = metrics.get("contentSize") {
                params["clip"] = serde_json::json!({
                    "x": 0,
                    "y": 0,
                    "width": content_size.get("width").and_then(|w| w.as_f64()).unwrap_or(1920.0),
                    "height": content_size.get("height").and_then(|h| h.as_f64()).unwrap_or(1080.0),
                    "scale": 1
                });
                params["captureBeyondViewport"] = serde_json::json!(true);
            }
        }

        let screenshot = client.send_command("Page.captureScreenshot", params)?;

        let base64_data = screenshot
            .get("data")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();

        let path = PathBuf::from(filepath);

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .context("Failed to create parent directory for screenshot")?;
            }
        }

        // Decode base64 and write to file
        let png_data = BASE64.decode(&base64_data)
            .context("Failed to decode screenshot data")?;
        std::fs::write(&path, &png_data)
            .context("Failed to write screenshot file")?;

        self.log(&format!("Screenshot saved to: {}", path.display()));

        Ok(ScreenshotResult {
            path,
            full_page,
        })
    }

    pub fn execute_script(&self, script: &str) -> Result<BrowserResult> {
        let mut client = self.get_client()?;

        // Enable Runtime
        client.send_command("Runtime.enable", serde_json::json!({}))?;

        let result = client.send_command(
            "Runtime.evaluate",
            serde_json::json!({
                "expression": script,
                "returnByValue": true,
                "awaitPromise": true
            }),
        )?;

        // Check for exceptions
        if let Some(exception) = result.get("exceptionDetails") {
            let error_text = exception
                .get("exception")
                .and_then(|e| e.get("description"))
                .and_then(|d| d.as_str())
                .unwrap_or("Unknown error");

            return Ok(BrowserResult {
                success: false,
                message: format!("Script execution failed: {}", error_text),
                data: Some(serde_json::json!({ "error": error_text })),
            });
        }

        let value = result
            .get("result")
            .and_then(|r| r.get("value"))
            .cloned()
            .unwrap_or(serde_json::json!(null));

        Ok(BrowserResult {
            success: true,
            message: "Script executed successfully".to_string(),
            data: Some(serde_json::json!({ "result": value })),
        })
    }

    pub fn stop_browser(&self) -> Result<BrowserResult> {
        let mut process_guard = self.chrome_process.lock().unwrap();

        if let Some(ref mut child) = *process_guard {
            child.kill().ok();
            child.wait().ok();
            *process_guard = None;

            Ok(BrowserResult {
                success: true,
                message: "Browser stopped".to_string(),
                data: None,
            })
        } else {
            Ok(BrowserResult {
                success: true,
                message: "Browser was not running".to_string(),
                data: None,
            })
        }
    }

    pub fn list_tabs(&self) -> Result<BrowserResult> {
        let http_url = format!("http://localhost:{}/json", self.config.port);

        let client = reqwest::blocking::Client::builder()
            .timeout(self.config.timeout)
            .build()?;

        let response = client
            .get(&http_url)
            .send()
            .context("Failed to connect to Chrome DevTools. Is the browser running?")?;

        let tabs: Vec<serde_json::Value> = response.json().context("Failed to parse Chrome tabs")?;

        // Filter to page-type tabs and extract useful info
        let tab_info: Vec<serde_json::Value> = tabs
            .iter()
            .filter(|t| t.get("type").and_then(|v| v.as_str()) == Some("page"))
            .map(|t| {
                serde_json::json!({
                    "id": t.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                    "url": t.get("url").and_then(|v| v.as_str()).unwrap_or(""),
                    "title": t.get("title").and_then(|v| v.as_str()).unwrap_or(""),
                })
            })
            .collect();

        Ok(BrowserResult {
            success: true,
            message: format!("Found {} tab(s)", tab_info.len()),
            data: Some(serde_json::json!({ "tabs": tab_info })),
        })
    }

    pub fn new_tab(&self, url: Option<&str>) -> Result<BrowserResult> {
        let http_url = format!(
            "http://localhost:{}/json/new?{}",
            self.config.port,
            url.unwrap_or("about:blank")
        );

        let client = reqwest::blocking::Client::builder()
            .timeout(self.config.timeout)
            .build()?;

        let response = client
            .put(&http_url)
            .send()
            .context("Failed to create new tab. Is the browser running?")?;

        let tab: serde_json::Value = response.json().context("Failed to parse new tab response")?;

        let tab_id = tab.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let tab_url = tab.get("url").and_then(|v| v.as_str()).unwrap_or("");

        Ok(BrowserResult {
            success: true,
            message: format!("Created new tab: {}", tab_id),
            data: Some(serde_json::json!({
                "id": tab_id,
                "url": tab_url
            })),
        })
    }

    pub fn close_tab(&self, tab_id: &str) -> Result<BrowserResult> {
        let http_url = format!("http://localhost:{}/json/close/{}", self.config.port, tab_id);

        let client = reqwest::blocking::Client::builder()
            .timeout(self.config.timeout)
            .build()?;

        let response = client
            .get(&http_url)
            .send()
            .context("Failed to close tab. Is the browser running?")?;

        let status = response.status();
        if status.is_success() {
            Ok(BrowserResult {
                success: true,
                message: format!("Closed tab: {}", tab_id),
                data: None,
            })
        } else {
            Ok(BrowserResult {
                success: false,
                message: format!("Failed to close tab: {} (status: {})", tab_id, status),
                data: None,
            })
        }
    }

    pub fn reload_page(&self, ignore_cache: bool) -> Result<BrowserResult> {
        let mut client = self.get_client()?;

        client.send_command(
            "Page.reload",
            serde_json::json!({ "ignoreCache": ignore_cache }),
        )?;

        Ok(BrowserResult {
            success: true,
            message: "Page reloaded".to_string(),
            data: Some(serde_json::json!({ "ignoreCache": ignore_cache })),
        })
    }

    pub fn go_back(&self) -> Result<BrowserResult> {
        let mut client = self.get_client()?;

        // Get navigation history
        let history = client.send_command("Page.getNavigationHistory", serde_json::json!({}))?;

        let current_index = history
            .get("currentIndex")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        if current_index <= 0 {
            return Ok(BrowserResult {
                success: false,
                message: "Cannot go back: already at the beginning of history".to_string(),
                data: None,
            });
        }

        let entries = history
            .get("entries")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("No history entries found"))?;

        let prev_entry = &entries[(current_index - 1) as usize];
        let entry_id = prev_entry
            .get("id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| anyhow::anyhow!("No entry ID found"))?;

        client.send_command(
            "Page.navigateToHistoryEntry",
            serde_json::json!({ "entryId": entry_id }),
        )?;

        Ok(BrowserResult {
            success: true,
            message: "Navigated back".to_string(),
            data: None,
        })
    }

    pub fn go_forward(&self) -> Result<BrowserResult> {
        let mut client = self.get_client()?;

        // Get navigation history
        let history = client.send_command("Page.getNavigationHistory", serde_json::json!({}))?;

        let current_index = history
            .get("currentIndex")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as usize;

        let entries = history
            .get("entries")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("No history entries found"))?;

        if current_index >= entries.len() - 1 {
            return Ok(BrowserResult {
                success: false,
                message: "Cannot go forward: already at the end of history".to_string(),
                data: None,
            });
        }

        let next_entry = &entries[current_index + 1];
        let entry_id = next_entry
            .get("id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| anyhow::anyhow!("No entry ID found"))?;

        client.send_command(
            "Page.navigateToHistoryEntry",
            serde_json::json!({ "entryId": entry_id }),
        )?;

        Ok(BrowserResult {
            success: true,
            message: "Navigated forward".to_string(),
            data: None,
        })
    }

    pub fn get_cookies(&self) -> Result<BrowserResult> {
        let mut client = self.get_client()?;

        client.send_command("Network.enable", serde_json::json!({}))?;
        let result = client.send_command("Network.getCookies", serde_json::json!({}))?;

        let cookies = result.get("cookies").cloned().unwrap_or(serde_json::json!([]));
        let count = cookies.as_array().map(|a| a.len()).unwrap_or(0);

        Ok(BrowserResult {
            success: true,
            message: format!("Retrieved {} cookie(s)", count),
            data: Some(serde_json::json!({ "cookies": cookies })),
        })
    }

    pub fn set_cookie(
        &self,
        name: &str,
        value: &str,
        domain: Option<&str>,
        path: Option<&str>,
        secure: bool,
        http_only: bool,
        expires: Option<f64>,
    ) -> Result<BrowserResult> {
        let mut client = self.get_client()?;

        client.send_command("Network.enable", serde_json::json!({}))?;

        let mut params = serde_json::json!({
            "name": name,
            "value": value,
            "secure": secure,
            "httpOnly": http_only,
        });

        if let Some(d) = domain {
            params["domain"] = serde_json::json!(d);
        }
        if let Some(p) = path {
            params["path"] = serde_json::json!(p);
        }
        if let Some(e) = expires {
            params["expires"] = serde_json::json!(e);
        }

        let result = client.send_command("Network.setCookie", params)?;

        let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);

        Ok(BrowserResult {
            success,
            message: if success {
                format!("Cookie '{}' set successfully", name)
            } else {
                format!("Failed to set cookie '{}'", name)
            },
            data: Some(serde_json::json!({ "name": name, "value": value })),
        })
    }

    pub fn find_element(&self, selector: &str) -> Result<BrowserResult> {
        let mut client = self.get_client()?;

        client.send_command("DOM.enable", serde_json::json!({}))?;

        let doc = client.send_command("DOM.getDocument", serde_json::json!({}))?;
        let root_node_id = doc
            .get("root")
            .and_then(|r| r.get("nodeId"))
            .and_then(|n| n.as_i64())
            .ok_or_else(|| anyhow::anyhow!("Could not get document root"))?;

        let result = client.send_command(
            "DOM.querySelector",
            serde_json::json!({
                "nodeId": root_node_id,
                "selector": selector
            }),
        )?;

        let node_id = result.get("nodeId").and_then(|n| n.as_i64()).unwrap_or(0);

        if node_id == 0 {
            return Ok(BrowserResult {
                success: false,
                message: format!("Element not found: {}", selector),
                data: None,
            });
        }

        // Get element attributes and box model
        let attrs = client.send_command("DOM.getAttributes", serde_json::json!({ "nodeId": node_id }));
        let box_model = client.send_command("DOM.getBoxModel", serde_json::json!({ "nodeId": node_id }));

        let attributes = attrs.ok().and_then(|a| a.get("attributes").cloned());
        let model = box_model.ok().and_then(|b| b.get("model").cloned());

        Ok(BrowserResult {
            success: true,
            message: format!("Found element: {}", selector),
            data: Some(serde_json::json!({
                "nodeId": node_id,
                "selector": selector,
                "attributes": attributes,
                "boxModel": model
            })),
        })
    }

    pub fn wait_for_element(&self, selector: &str, timeout_ms: u64) -> Result<BrowserResult> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_millis(timeout_ms);

        while start.elapsed() < timeout {
            match self.find_element(selector) {
                Ok(result) if result.success => return Ok(result),
                _ => std::thread::sleep(Duration::from_millis(100)),
            }
        }

        Ok(BrowserResult {
            success: false,
            message: format!(
                "Timeout waiting for element '{}' after {}ms",
                selector, timeout_ms
            ),
            data: None,
        })
    }

    pub fn click_element(&self, selector: &str) -> Result<BrowserResult> {
        let mut client = self.get_client()?;

        client.send_command("DOM.enable", serde_json::json!({}))?;
        client.send_command("Runtime.enable", serde_json::json!({}))?;

        // Get document and find element
        let doc = client.send_command("DOM.getDocument", serde_json::json!({}))?;
        let root_node_id = doc
            .get("root")
            .and_then(|r| r.get("nodeId"))
            .and_then(|n| n.as_i64())
            .ok_or_else(|| anyhow::anyhow!("Could not get document root"))?;

        let result = client.send_command(
            "DOM.querySelector",
            serde_json::json!({
                "nodeId": root_node_id,
                "selector": selector
            }),
        )?;

        let node_id = result.get("nodeId").and_then(|n| n.as_i64()).unwrap_or(0);

        if node_id == 0 {
            return Ok(BrowserResult {
                success: false,
                message: format!("Element not found: {}", selector),
                data: None,
            });
        }

        // Get box model to find click coordinates
        let box_model = client.send_command("DOM.getBoxModel", serde_json::json!({ "nodeId": node_id }))?;

        let content = box_model
            .get("model")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_array())
            .ok_or_else(|| anyhow::anyhow!("Could not get element position"))?;

        // Calculate center point of the element
        // content is [x1, y1, x2, y2, x3, y3, x4, y4] - 4 corners
        let x = (content.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0)
            + content.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0))
            / 2.0;
        let y = (content.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0)
            + content.get(5).and_then(|v| v.as_f64()).unwrap_or(0.0))
            / 2.0;

        // Scroll element into view first
        let _ = client.send_command(
            "DOM.scrollIntoViewIfNeeded",
            serde_json::json!({ "nodeId": node_id }),
        );

        // Small delay for scroll to complete
        std::thread::sleep(Duration::from_millis(100));

        // Dispatch mouse events
        client.send_command(
            "Input.dispatchMouseEvent",
            serde_json::json!({
                "type": "mousePressed",
                "x": x,
                "y": y,
                "button": "left",
                "clickCount": 1
            }),
        )?;

        client.send_command(
            "Input.dispatchMouseEvent",
            serde_json::json!({
                "type": "mouseReleased",
                "x": x,
                "y": y,
                "button": "left",
                "clickCount": 1
            }),
        )?;

        Ok(BrowserResult {
            success: true,
            message: format!("Clicked element: {}", selector),
            data: Some(serde_json::json!({
                "selector": selector,
                "x": x,
                "y": y
            })),
        })
    }

    pub fn type_text(&self, text: &str, selector: Option<&str>) -> Result<BrowserResult> {
        let mut client = self.get_client()?;

        // If selector provided, click to focus first
        if let Some(sel) = selector {
            // Use our click_element logic inline to focus the element
            client.send_command("DOM.enable", serde_json::json!({}))?;

            let doc = client.send_command("DOM.getDocument", serde_json::json!({}))?;
            let root_node_id = doc
                .get("root")
                .and_then(|r| r.get("nodeId"))
                .and_then(|n| n.as_i64())
                .ok_or_else(|| anyhow::anyhow!("Could not get document root"))?;

            let result = client.send_command(
                "DOM.querySelector",
                serde_json::json!({
                    "nodeId": root_node_id,
                    "selector": sel
                }),
            )?;

            let node_id = result.get("nodeId").and_then(|n| n.as_i64()).unwrap_or(0);

            if node_id == 0 {
                return Ok(BrowserResult {
                    success: false,
                    message: format!("Element not found: {}", sel),
                    data: None,
                });
            }

            // Focus the element
            client.send_command("DOM.focus", serde_json::json!({ "nodeId": node_id }))?;

            std::thread::sleep(Duration::from_millis(50));
        }

        // Type each character
        for c in text.chars() {
            client.send_command(
                "Input.dispatchKeyEvent",
                serde_json::json!({
                    "type": "keyDown",
                    "text": c.to_string(),
                }),
            )?;

            client.send_command(
                "Input.dispatchKeyEvent",
                serde_json::json!({
                    "type": "keyUp",
                    "text": c.to_string(),
                }),
            )?;
        }

        Ok(BrowserResult {
            success: true,
            message: format!("Typed {} characters", text.len()),
            data: Some(serde_json::json!({
                "text": text,
                "selector": selector
            })),
        })
    }

    pub fn keyboard_press(&self, key: &str, modifiers: Option<&str>) -> Result<BrowserResult> {
        let mut client = self.get_client()?;

        // Map common key names to key codes
        let (key_code, code, key_text) = match key.to_lowercase().as_str() {
            "enter" | "return" => (13, "Enter", "\r"),
            "tab" => (9, "Tab", "\t"),
            "escape" | "esc" => (27, "Escape", ""),
            "backspace" => (8, "Backspace", ""),
            "delete" => (46, "Delete", ""),
            "arrowup" | "up" => (38, "ArrowUp", ""),
            "arrowdown" | "down" => (40, "ArrowDown", ""),
            "arrowleft" | "left" => (37, "ArrowLeft", ""),
            "arrowright" | "right" => (39, "ArrowRight", ""),
            "home" => (36, "Home", ""),
            "end" => (35, "End", ""),
            "pageup" => (33, "PageUp", ""),
            "pagedown" => (34, "PageDown", ""),
            "space" => (32, "Space", " "),
            _ => {
                // For single character keys
                if key.len() == 1 {
                    let c = key.chars().next().unwrap();
                    (c as i32, key, key)
                } else {
                    return Ok(BrowserResult {
                        success: false,
                        message: format!("Unknown key: {}", key),
                        data: None,
                    });
                }
            }
        };

        // Parse modifiers
        let mut modifier_flags = 0;
        if let Some(mods) = modifiers {
            for m in mods.split('+') {
                match m.trim().to_lowercase().as_str() {
                    "alt" => modifier_flags |= 1,
                    "ctrl" | "control" => modifier_flags |= 2,
                    "meta" | "cmd" | "command" => modifier_flags |= 4,
                    "shift" => modifier_flags |= 8,
                    _ => {}
                }
            }
        }

        // Key down
        client.send_command(
            "Input.dispatchKeyEvent",
            serde_json::json!({
                "type": "keyDown",
                "windowsVirtualKeyCode": key_code,
                "code": code,
                "key": key,
                "text": key_text,
                "modifiers": modifier_flags
            }),
        )?;

        // Key up
        client.send_command(
            "Input.dispatchKeyEvent",
            serde_json::json!({
                "type": "keyUp",
                "windowsVirtualKeyCode": key_code,
                "code": code,
                "key": key,
                "modifiers": modifier_flags
            }),
        )?;

        Ok(BrowserResult {
            success: true,
            message: format!("Pressed key: {}", key),
            data: Some(serde_json::json!({
                "key": key,
                "modifiers": modifiers
            })),
        })
    }

    pub fn print_to_pdf(&self, filepath: &str, landscape: bool, print_background: bool) -> Result<BrowserResult> {
        let mut client = self.get_client()?;

        client.send_command("Page.enable", serde_json::json!({}))?;

        let result = client.send_command(
            "Page.printToPDF",
            serde_json::json!({
                "landscape": landscape,
                "printBackground": print_background,
                "preferCSSPageSize": true
            }),
        )?;

        let base64_data = result
            .get("data")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();

        let path = std::path::PathBuf::from(filepath);

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .context("Failed to create parent directory for PDF")?;
            }
        }

        // Decode and write
        let pdf_data = BASE64.decode(&base64_data).context("Failed to decode PDF data")?;
        std::fs::write(&path, &pdf_data).context("Failed to write PDF file")?;

        self.log(&format!("PDF saved to: {}", path.display()));

        Ok(BrowserResult {
            success: true,
            message: format!("PDF saved to {}", path.display()),
            data: Some(serde_json::json!({
                "path": path.display().to_string(),
                "landscape": landscape,
                "printBackground": print_background
            })),
        })
    }

    pub fn is_available(&self) -> BrowserResult {
        let chrome_path = std::path::Path::new(&self.config.chrome_path);

        // Check if the Chrome executable exists
        if !chrome_path.exists() {
            return BrowserResult {
                success: false,
                message: format!(
                    "Chrome not found at configured path: {}",
                    self.config.chrome_path
                ),
                data: Some(serde_json::json!({
                    "available": false,
                    "chrome_path": self.config.chrome_path,
                    "reason": "executable_not_found"
                })),
            };
        }

        // Try to get Chrome version to verify it's executable
        match Command::new(&self.config.chrome_path)
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    let version = String::from_utf8_lossy(&output.stdout)
                        .trim()
                        .to_string();

                    BrowserResult {
                        success: true,
                        message: format!("Chrome is available: {}", version),
                        data: Some(serde_json::json!({
                            "available": true,
                            "chrome_path": self.config.chrome_path,
                            "version": version
                        })),
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr)
                        .trim()
                        .to_string();

                    BrowserResult {
                        success: false,
                        message: format!("Chrome executable failed: {}", stderr),
                        data: Some(serde_json::json!({
                            "available": false,
                            "chrome_path": self.config.chrome_path,
                            "reason": "execution_failed",
                            "error": stderr
                        })),
                    }
                }
            }
            Err(e) => BrowserResult {
                success: false,
                message: format!("Failed to execute Chrome: {}", e),
                data: Some(serde_json::json!({
                    "available": false,
                    "chrome_path": self.config.chrome_path,
                    "reason": "execution_error",
                    "error": e.to_string()
                })),
            },
        }
    }
}

impl Drop for BrowserServer {
    fn drop(&mut self) {
        // Stop Chrome when server is dropped
        let mut process_guard = self.chrome_process.lock().unwrap();
        if let Some(ref mut child) = *process_guard {
            child.kill().ok();
            child.wait().ok();
        }
    }
}

impl McpServer for BrowserServer {
    fn name(&self) -> &str {
        "mcpz-browser"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn verbose(&self) -> bool {
        self.config.verbose
    }

    fn tools(&self) -> Vec<McpTool> {
        vec![
            McpTool {
                name: "is_available".to_string(),
                description: "Check if Chrome is installed and usable. Returns Chrome version if available.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            McpTool {
                name: "start_browser".to_string(),
                description: "Start a Chrome browser instance with remote debugging enabled"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "headless": {
                            "type": "boolean",
                            "description": "Run Chrome in headless mode (no visible window). Default: false",
                            "default": false
                        }
                    },
                    "required": []
                }),
            },
            McpTool {
                name: "stop_browser".to_string(),
                description: "Stop the running Chrome browser instance".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            McpTool {
                name: "open_url".to_string(),
                description: "Navigate to a URL in the browser".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "The URL to navigate to"
                        },
                        "wait_time": {
                            "type": "integer",
                            "description": "Seconds to wait after page load for dynamic content (default: 3)",
                            "default": 3
                        },
                        "tab_id": {
                            "type": "string",
                            "description": "Target tab ID to navigate (from list_tabs). If not specified, uses current tab."
                        }
                    },
                    "required": ["url"]
                }),
            },
            McpTool {
                name: "get_page".to_string(),
                description: "Get the full HTML content of the current page".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            McpTool {
                name: "get_page_as_markdown".to_string(),
                description: "Get the current page content converted to markdown".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            McpTool {
                name: "save_page_contents".to_string(),
                description: "Save the full rendered DOM (HTML) of the current page to a file".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "filepath": {
                            "type": "string",
                            "description": "File path where the HTML should be saved (e.g., '/tmp/page.html')"
                        }
                    },
                    "required": ["filepath"]
                }),
            },
            McpTool {
                name: "get_screenshot".to_string(),
                description: "Capture a screenshot of the current page and save to the specified file path".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "filepath": {
                            "type": "string",
                            "description": "The file path where the screenshot should be saved (e.g., '/tmp/screenshot.png')"
                        },
                        "full_page": {
                            "type": "boolean",
                            "description": "Capture the full page or just the viewport (default: false)",
                            "default": false
                        }
                    },
                    "required": ["filepath"]
                }),
            },
            McpTool {
                name: "execute_script".to_string(),
                description: "Execute JavaScript code in the browser context".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "script": {
                            "type": "string",
                            "description": "JavaScript code to execute"
                        }
                    },
                    "required": ["script"]
                }),
            },
            // Tab management tools
            McpTool {
                name: "list_tabs".to_string(),
                description: "List all open browser tabs with their IDs, URLs, and titles".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            McpTool {
                name: "new_tab".to_string(),
                description: "Open a new browser tab, optionally navigating to a URL".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "URL to open in the new tab (default: about:blank)"
                        }
                    },
                    "required": []
                }),
            },
            McpTool {
                name: "close_tab".to_string(),
                description: "Close a browser tab by its ID".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "tab_id": {
                            "type": "string",
                            "description": "The ID of the tab to close (get from list_tabs)"
                        }
                    },
                    "required": ["tab_id"]
                }),
            },
            // Navigation tools
            McpTool {
                name: "reload_page".to_string(),
                description: "Reload the current page".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "ignore_cache": {
                            "type": "boolean",
                            "description": "If true, bypass the cache (hard reload). Default: false",
                            "default": false
                        }
                    },
                    "required": []
                }),
            },
            McpTool {
                name: "go_back".to_string(),
                description: "Navigate back in browser history".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            McpTool {
                name: "go_forward".to_string(),
                description: "Navigate forward in browser history".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            // Cookie tools
            McpTool {
                name: "get_cookies".to_string(),
                description: "Get all cookies for the current page".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            McpTool {
                name: "set_cookie".to_string(),
                description: "Set a cookie".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Cookie name"
                        },
                        "value": {
                            "type": "string",
                            "description": "Cookie value"
                        },
                        "domain": {
                            "type": "string",
                            "description": "Cookie domain (optional)"
                        },
                        "path": {
                            "type": "string",
                            "description": "Cookie path (optional, default: /)"
                        },
                        "secure": {
                            "type": "boolean",
                            "description": "Secure flag (default: false)",
                            "default": false
                        },
                        "http_only": {
                            "type": "boolean",
                            "description": "HttpOnly flag (default: false)",
                            "default": false
                        },
                        "expires": {
                            "type": "number",
                            "description": "Expiration timestamp in seconds since epoch (optional)"
                        }
                    },
                    "required": ["name", "value"]
                }),
            },
            // DOM interaction tools
            McpTool {
                name: "find_element".to_string(),
                description: "Find an element by CSS selector and return its info".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "selector": {
                            "type": "string",
                            "description": "CSS selector to find the element"
                        }
                    },
                    "required": ["selector"]
                }),
            },
            McpTool {
                name: "wait_for_element".to_string(),
                description: "Wait for an element to appear on the page".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "selector": {
                            "type": "string",
                            "description": "CSS selector to wait for"
                        },
                        "timeout": {
                            "type": "integer",
                            "description": "Maximum time to wait in milliseconds (default: 5000)",
                            "default": 5000
                        }
                    },
                    "required": ["selector"]
                }),
            },
            McpTool {
                name: "click_element".to_string(),
                description: "Click an element by CSS selector".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "selector": {
                            "type": "string",
                            "description": "CSS selector of the element to click"
                        }
                    },
                    "required": ["selector"]
                }),
            },
            McpTool {
                name: "type_text".to_string(),
                description: "Type text into an element or the currently focused element".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "text": {
                            "type": "string",
                            "description": "The text to type"
                        },
                        "selector": {
                            "type": "string",
                            "description": "CSS selector of element to focus before typing (optional)"
                        }
                    },
                    "required": ["text"]
                }),
            },
            McpTool {
                name: "keyboard_press".to_string(),
                description: "Press a keyboard key (Enter, Tab, Escape, arrows, etc.)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "key": {
                            "type": "string",
                            "description": "Key to press: Enter, Tab, Escape, Backspace, Delete, ArrowUp, ArrowDown, ArrowLeft, ArrowRight, Home, End, PageUp, PageDown, Space, or a single character"
                        },
                        "modifiers": {
                            "type": "string",
                            "description": "Modifier keys separated by +: Ctrl, Alt, Shift, Meta (e.g., 'Ctrl+Shift')"
                        }
                    },
                    "required": ["key"]
                }),
            },
            // PDF tool
            McpTool {
                name: "print_to_pdf".to_string(),
                description: "Save the current page as a PDF file".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "filepath": {
                            "type": "string",
                            "description": "File path where the PDF should be saved"
                        },
                        "landscape": {
                            "type": "boolean",
                            "description": "Use landscape orientation (default: false)",
                            "default": false
                        },
                        "print_background": {
                            "type": "boolean",
                            "description": "Print background graphics (default: true)",
                            "default": true
                        }
                    },
                    "required": ["filepath"]
                }),
            },
        ]
    }

    fn call_tool(&self, name: &str, arguments: &serde_json::Value) -> Result<serde_json::Value> {
        let result = match name {
            "is_available" => Ok(self.is_available()),
            "start_browser" => {
                let headless = arguments
                    .get("headless")
                    .and_then(|h| h.as_bool())
                    .unwrap_or(false);
                self.start_browser(headless)
            }
            "stop_browser" => self.stop_browser(),
            "open_url" => {
                let url = arguments
                    .get("url")
                    .and_then(|u| u.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing url argument"))?;

                let wait_time = arguments
                    .get("wait_time")
                    .and_then(|w| w.as_u64())
                    .unwrap_or(3);

                let tab_id = arguments.get("tab_id").and_then(|t| t.as_str());

                self.open_url(url, wait_time, tab_id)
            }
            "get_page" => self.get_page(),
            "get_page_as_markdown" => self.get_page_as_markdown(),
            "save_page_contents" => {
                let filepath = arguments
                    .get("filepath")
                    .and_then(|f| f.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing filepath argument"))?;
                self.save_page_contents(filepath)
            }
            "get_screenshot" => {
                let filepath = arguments
                    .get("filepath")
                    .and_then(|f| f.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing filepath argument"))?;

                let full_page = arguments
                    .get("full_page")
                    .and_then(|f| f.as_bool())
                    .unwrap_or(false);

                // Screenshot has special handling - return early
                return match self.get_screenshot(filepath, full_page) {
                    Ok(result) => {
                        Ok(text_content(&serde_json::to_string_pretty(&serde_json::json!({
                            "success": true,
                            "message": "Screenshot saved to file",
                            "path": result.path.display().to_string(),
                            "full_page": result.full_page
                        }))?))
                    }
                    Err(e) => Ok(error_content(&e.to_string())),
                };
            }
            "execute_script" => {
                let script = arguments
                    .get("script")
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing script argument"))?;

                self.execute_script(script)
            }
            // Tab management
            "list_tabs" => self.list_tabs(),
            "new_tab" => {
                let url = arguments.get("url").and_then(|u| u.as_str());
                self.new_tab(url)
            }
            "close_tab" => {
                let tab_id = arguments
                    .get("tab_id")
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing tab_id argument"))?;
                self.close_tab(tab_id)
            }
            // Navigation
            "reload_page" => {
                let ignore_cache = arguments
                    .get("ignore_cache")
                    .and_then(|i| i.as_bool())
                    .unwrap_or(false);
                self.reload_page(ignore_cache)
            }
            "go_back" => self.go_back(),
            "go_forward" => self.go_forward(),
            // Cookies
            "get_cookies" => self.get_cookies(),
            "set_cookie" => {
                let name = arguments
                    .get("name")
                    .and_then(|n| n.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing name argument"))?;
                let value = arguments
                    .get("value")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing value argument"))?;
                let domain = arguments.get("domain").and_then(|d| d.as_str());
                let path = arguments.get("path").and_then(|p| p.as_str());
                let secure = arguments
                    .get("secure")
                    .and_then(|s| s.as_bool())
                    .unwrap_or(false);
                let http_only = arguments
                    .get("http_only")
                    .and_then(|h| h.as_bool())
                    .unwrap_or(false);
                let expires = arguments.get("expires").and_then(|e| e.as_f64());
                self.set_cookie(name, value, domain, path, secure, http_only, expires)
            }
            // DOM interaction
            "find_element" => {
                let selector = arguments
                    .get("selector")
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing selector argument"))?;
                self.find_element(selector)
            }
            "wait_for_element" => {
                let selector = arguments
                    .get("selector")
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing selector argument"))?;
                let timeout = arguments
                    .get("timeout")
                    .and_then(|t| t.as_u64())
                    .unwrap_or(5000);
                self.wait_for_element(selector, timeout)
            }
            "click_element" => {
                let selector = arguments
                    .get("selector")
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing selector argument"))?;
                self.click_element(selector)
            }
            "type_text" => {
                let text = arguments
                    .get("text")
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing text argument"))?;
                let selector = arguments.get("selector").and_then(|s| s.as_str());
                self.type_text(text, selector)
            }
            "keyboard_press" => {
                let key = arguments
                    .get("key")
                    .and_then(|k| k.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing key argument"))?;
                let modifiers = arguments.get("modifiers").and_then(|m| m.as_str());
                self.keyboard_press(key, modifiers)
            }
            // PDF
            "print_to_pdf" => {
                let filepath = arguments
                    .get("filepath")
                    .and_then(|f| f.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing filepath argument"))?;
                let landscape = arguments
                    .get("landscape")
                    .and_then(|l| l.as_bool())
                    .unwrap_or(false);
                let print_background = arguments
                    .get("print_background")
                    .and_then(|p| p.as_bool())
                    .unwrap_or(true);
                self.print_to_pdf(filepath, landscape, print_background)
            }
            _ => {
                return Ok(error_content(&format!("Unknown tool: {}", name)));
            }
        };

        match result {
            Ok(browser_result) => {
                let result_json = serde_json::to_string_pretty(&browser_result)?;
                if browser_result.success {
                    Ok(text_content(&result_json))
                } else {
                    Ok(error_content(&result_json))
                }
            }
            Err(e) => Ok(error_content(&e.to_string())),
        }
    }
}

/// Simple HTML to Markdown converter
fn html_to_markdown(html: &str) -> String {
    // Remove script and style tags
    let mut result = html.to_string();

    // Remove script tags
    while let Some(start) = result.find("<script") {
        if let Some(end) = result[start..].find("</script>") {
            result = format!("{}{}", &result[..start], &result[start + end + 9..]);
        } else {
            break;
        }
    }

    // Remove style tags
    while let Some(start) = result.find("<style") {
        if let Some(end) = result[start..].find("</style>") {
            result = format!("{}{}", &result[..start], &result[start + end + 8..]);
        } else {
            break;
        }
    }

    // Simple tag replacements
    let replacements = [
        ("<h1>", "# "),
        ("</h1>", "\n\n"),
        ("<h1 ", "# "),
        ("<h2>", "## "),
        ("</h2>", "\n\n"),
        ("<h2 ", "## "),
        ("<h3>", "### "),
        ("</h3>", "\n\n"),
        ("<h3 ", "### "),
        ("<h4>", "#### "),
        ("</h4>", "\n\n"),
        ("<h4 ", "#### "),
        ("<h5>", "##### "),
        ("</h5>", "\n\n"),
        ("<h5 ", "##### "),
        ("<h6>", "###### "),
        ("</h6>", "\n\n"),
        ("<h6 ", "###### "),
        ("<p>", "\n"),
        ("</p>", "\n\n"),
        ("<p ", "\n"),
        ("<br>", "\n"),
        ("<br/>", "\n"),
        ("<br />", "\n"),
        ("<li>", "- "),
        ("</li>", "\n"),
        ("<li ", "- "),
        ("<strong>", "**"),
        ("</strong>", "**"),
        ("<b>", "**"),
        ("</b>", "**"),
        ("<em>", "*"),
        ("</em>", "*"),
        ("<i>", "*"),
        ("</i>", "*"),
        ("<code>", "`"),
        ("</code>", "`"),
        ("<hr>", "\n---\n"),
        ("<hr/>", "\n---\n"),
        ("<hr />", "\n---\n"),
    ];

    for (from, to) in replacements {
        result = result.replace(from, to);
    }

    // Remove all remaining HTML tags
    let mut cleaned = String::new();
    let mut in_tag = false;
    let mut in_attr_value = false;

    for c in result.chars() {
        match c {
            '<' if !in_attr_value => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                in_attr_value = false;
            }
            '"' if in_tag => in_attr_value = !in_attr_value,
            _ if !in_tag => cleaned.push(c),
            _ => {}
        }
    }

    // Decode HTML entities
    let entities = [
        ("&amp;", "&"),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&apos;", "'"),
        ("&nbsp;", " "),
        ("&#39;", "'"),
        ("&#x27;", "'"),
    ];

    for (entity, char) in entities {
        cleaned = cleaned.replace(entity, char);
    }

    // Clean up whitespace
    let lines: Vec<&str> = cleaned.lines().map(|l| l.trim()).collect();
    let mut final_result = String::new();
    let mut prev_empty = false;

    for line in lines {
        if line.is_empty() {
            if !prev_empty {
                final_result.push('\n');
                prev_empty = true;
            }
        } else {
            final_result.push_str(line);
            final_result.push('\n');
            prev_empty = false;
        }
    }

    final_result.trim().to_string()
}

/// Run the browser MCP server
pub fn run_browser_server(config: BrowserServerConfig) -> Result<()> {
    if config.verbose {
        eprintln!("[mcpz] Browser server configuration:");
        eprintln!("[mcpz]   Chrome path: {}", config.chrome_path);
        eprintln!("[mcpz]   DevTools port: {}", config.port);
        eprintln!("[mcpz]   User data dir: {}", config.user_data_dir.display());
        eprintln!("[mcpz]   HTTP mode: {}", config.http_mode);
        eprintln!("[mcpz]   Timeout: {:?}", config.timeout);
    }

    let server = BrowserServer::new(config);
    server.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_config_new() {
        let config = BrowserServerConfig::new(9222, None, 30, false);
        assert_eq!(config.port, 9222);
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!(!config.verbose);
        assert!(!config.http_mode);
        assert!(config.user_data_dir.to_string_lossy().contains("browser-9222"));
    }

    #[test]
    fn test_browser_config_custom_chrome() {
        let config = BrowserServerConfig::new(
            9222,
            Some("/usr/bin/chromium".to_string()),
            60,
            true,
        );
        assert_eq!(config.chrome_path, "/usr/bin/chromium");
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert!(config.verbose);
    }

    #[test]
    fn test_browser_config_http_mode() {
        let config = BrowserServerConfig::new(9222, None, 30, false)
            .with_http_mode(true);
        assert!(config.http_mode);

        let config2 = BrowserServerConfig::new(9222, None, 30, false)
            .with_http_mode(false);
        assert!(!config2.http_mode);
    }

    #[test]
    fn test_browser_server_tools() {
        let config = BrowserServerConfig::new(9222, None, 30, false);
        let server = BrowserServer::new(config);
        let tools = server.tools();

        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

        assert!(tool_names.contains(&"is_available"));
        assert!(tool_names.contains(&"start_browser"));
        assert!(tool_names.contains(&"stop_browser"));
        assert!(tool_names.contains(&"open_url"));
        assert!(tool_names.contains(&"get_page"));
        assert!(tool_names.contains(&"get_page_as_markdown"));
        assert!(tool_names.contains(&"get_screenshot"));
        assert!(tool_names.contains(&"execute_script"));
    }

    #[test]
    fn test_is_available_with_invalid_path() {
        let config = BrowserServerConfig::new(
            9222,
            Some("/nonexistent/path/to/chrome".to_string()),
            30,
            false,
        );
        let server = BrowserServer::new(config);
        let result = server.is_available();

        assert!(!result.success);
        assert!(result.message.contains("not found"));
        assert!(result.data.is_some());
        let data = result.data.unwrap();
        assert_eq!(data["available"], false);
        assert_eq!(data["reason"], "executable_not_found");
    }

    #[test]
    fn test_browser_server_initialize() {
        let config = BrowserServerConfig::new(9222, None, 30, false);
        let server = BrowserServer::new(config);
        let result = server.handle_initialize();

        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert_eq!(result["serverInfo"]["name"], "mcpz-browser");
    }

    #[test]
    fn test_html_to_markdown_headings() {
        assert!(html_to_markdown("<h1>Title</h1>").contains("# Title"));
        assert!(html_to_markdown("<h2>Section</h2>").contains("## Section"));
        assert!(html_to_markdown("<h3>Subsection</h3>").contains("### Subsection"));
    }

    #[test]
    fn test_html_to_markdown_formatting() {
        assert!(html_to_markdown("<strong>bold</strong>").contains("**bold**"));
        assert!(html_to_markdown("<em>italic</em>").contains("*italic*"));
        assert!(html_to_markdown("<code>code</code>").contains("`code`"));
    }

    #[test]
    fn test_html_to_markdown_removes_script() {
        let html = "<p>Before</p><script>alert('test')</script><p>After</p>";
        let md = html_to_markdown(html);
        assert!(!md.contains("alert"));
        assert!(md.contains("Before"));
        assert!(md.contains("After"));
    }

    #[test]
    fn test_html_to_markdown_entities() {
        assert!(html_to_markdown("&amp;").contains("&"));
        assert!(html_to_markdown("&lt;").contains("<"));
        assert!(html_to_markdown("&gt;").contains(">"));
    }

    #[test]
    fn test_browser_result_serialization() {
        let result = BrowserResult {
            success: true,
            message: "Test".to_string(),
            data: Some(serde_json::json!({"key": "value"})),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"message\":\"Test\""));
        assert!(json.contains("\"key\":\"value\""));
    }

    #[test]
    fn test_browser_result_without_data() {
        let result = BrowserResult {
            success: true,
            message: "Test".to_string(),
            data: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(!json.contains("data"));
    }
}
