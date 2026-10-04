use tauri::{
    plugin::{Builder, PluginHandle, TauriPlugin},
    Manager, Runtime,
};

tauri::ios_plugin_binding!(init_plugin_file_actions);

pub struct FileActions<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> FileActions<R> {
    pub fn open(
        &self,
        paths: Vec<String>,
        preview: bool,
    ) -> Result<(), tauri::plugin::mobile::PluginInvokeError> {
        #[derive(serde::Serialize)]
        struct Request {
            paths: Vec<String>,
            preview: bool,
        }
        self.0
            .run_mobile_plugin("openFiles", Request { paths, preview })
    }
}

pub trait FileActionsExt<R: Runtime> {
    fn file_actions(&self) -> &FileActions<R>;
}

impl<R: Runtime, T: Manager<R>> FileActionsExt<R> for T {
    fn file_actions(&self) -> &FileActions<R> {
        self.state::<FileActions<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("file-actions")
        .setup(|app, api| {
            app.manage(FileActions(
                api.register_ios_plugin(init_plugin_file_actions)?,
            ));
            Ok(())
        })
        .build()
}
