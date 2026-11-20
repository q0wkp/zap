wit_bindgen::generate!({
    path: "../../../wit",
    world: "plugin",
});

use zap_ng::plugin::host;

struct Probe;

impl Guest for Probe {
    fn handle(hook: String, payload: String) -> Result<String, String> {
        match hook.as_str() {
            "echo" => {
                host::log(host::LogLevel::Info, &payload);
                Ok(format!("{payload}|{}", host::plugin_config()))
            }
            "read" => std::fs::read_to_string(&payload).map_err(|error| error.to_string()),
            "write" => {
                let (path, text) = payload.split_once('=').ok_or("expected path=text")?;
                std::fs::write(path, text).map_err(|error| error.to_string())?;
                Ok(String::new())
            }
            "secret" => {
                host::write_secret("key", &payload)?;
                Ok(host::read_secret("old-service", "key")?.unwrap_or_default())
            }
            "spin" => loop {
                std::hint::spin_loop();
            },
            other => Err(format!("unknown hook {other}")),
        }
    }
}

export!(Probe);
