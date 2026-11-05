// TurtleClient native launcher. Derived from TurtleClient/LiquidLauncher (GPL-3.0-or-later).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod native;
use tauri::Manager;

fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|a| a == "--self-check") {
        let Some(output) = args.get(1) else {
            std::process::exit(2)
        };
        let result = native::Backend::beside_executable().and_then(|b| {
            b.install()?;
            b.snapshot(&native::Progress::default())
        });
        let value = match result {
            Ok(v) => serde_json::json!({"ok":true,"state":v}),
            Err(e) => serde_json::json!({"ok":false,"error":e}),
        };
        let _ = std::fs::write(output, serde_json::to_vec_pretty(&value).unwrap());
        std::process::exit(if value["ok"] == true { 0 } else { 1 });
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            app.manage(native::Shared::new(native::Backend::new(
                app.path().resource_dir()?.join("payload"),
            )?));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            native::get_state,
            native::start_action,
            native::cancel_action,
            native::open_location,
            native::open_sign_in,
            native::launcher::save_modules,
            native::launcher::read_cape,
            native::launcher::upload_cape,
            native::skin::read_player_skin
        ])
        .build(tauri::generate_context!())
        .expect("Turtle Launcher could not start")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit)
                && let Some(shared) = app.try_state::<native::Shared>()
            {
                shared.cancel_non_game();
            }
        });
}
