use axum::{
    Json, Router,
    response::Html,
    routing::{get, post},
};
use local_ip_address::list_afinet_netifas;
use serde::Deserialize;
use std::{env, net::SocketAddr};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, SendInput,
    VIRTUAL_KEY,
};

const DEFAULT_PORT: u16 = 5566;

fn get_port_from_exe_name() -> u16 {
    let exe_path = env::current_exe().unwrap_or_default();
    let exe_name = exe_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let chars: Vec<char> = exe_name.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let mut j = i;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            if let Ok(port) = chars[i..j].iter().collect::<String>().parse::<u16>() {
                return port;
            }
            i = j;
        } else {
            i += 1;
        }
    }
    DEFAULT_PORT
}

fn send_text(text: &str) {
    let utf16: Vec<u16> = text.encode_utf16().collect();
    let mut inputs: Vec<INPUT> = Vec::with_capacity(utf16.len() * 2);

    for &code_unit in &utf16 {
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: code_unit,
                    dwFlags: KEYEVENTF_UNICODE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: code_unit,
                    dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
    }

    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

#[derive(Deserialize)]
struct TypeRequest {
    text: String,
}

async fn handle_type(Json(req): Json<TypeRequest>) -> &'static str {
    send_text(&req.text);
    "ok"
}

const HTML: &str = r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Input Server</title>
  <style>
    body { font-family: sans-serif; max-width: 640px; margin: 10px auto; padding: 0 10px; }
    h2 { margin-bottom: 16px; }
    textarea {
      width: 100%; height: 200px; font-size: 16px;
      padding: 8px; box-sizing: border-box; border-radius: 4px;
      border: 1px solid #ccc; resize: vertical;
    }
    .buttons { margin-top: 10px; display: flex; gap: 10px; }
    button {
      flex: 1; padding: 14px; font-size: 16px;
      cursor: pointer; border: none; border-radius: 4px; color: white;
    }
    #send-btn  { background: #4CAF50; }
    #clear-btn { background: #f44336; }
    #status { margin-top: 10px; min-height: 20px; color: gray; font-size: 14px; }
  </style>
</head>
<body>
  <h2>Input Server</h2>
  <textarea id="text" placeholder="在此输入要发送的文字…"></textarea>
  <div class="buttons">
    <button id="clear-btn" onclick="clearText()">清空</button>
    <button id="send-btn"  onclick="sendText()">发送</button>
  </div>
  <div id="status"></div>
  <script>
    async function sendText() {
      const text = document.getElementById('text').value;
      if (!text) return;
      const status = document.getElementById('status');
      status.textContent = '发送中…';
      try {
        const res = await fetch('/type', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ text })
        });
        status.textContent = res.ok ? '✓ 发送成功' : '✗ 发送失败：' + res.status;
      } catch (e) {
        status.textContent = '✗ 错误：' + e.message;
      }
    }

    function clearText() {
      document.getElementById('text').value = '';
      document.getElementById('status').textContent = '';
    }

    // Ctrl+Enter 快捷发送
    document.getElementById('text').addEventListener('keydown', e => {
      if (e.ctrlKey && e.key === 'Enter') sendText();
    });
  </script>
</body>
</html>"#;

async fn handle_index() -> Html<&'static str> {
    Html(HTML)
}

#[tokio::main]
async fn main() {
    let port = get_port_from_exe_name();
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let app = Router::new()
        .route("/", get(handle_index))
        .route("/type", post(handle_type));

    println!("Input server started. Access via:");
    println!("  http://localhost:{port}");
    if let Ok(interfaces) = list_afinet_netifas() {
        for (iface, ip) in interfaces {
            if !ip.is_loopback() {
                println!("  http://{ip}:{port}  ({iface})");
            }
        }
    }
    println!();
    println!(
        "Tip: rename the exe to include a port number (e.g. input_server_8080.exe) to change the port."
    );

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
