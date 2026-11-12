//! Yappuccino's Windows installer. Run as built, with an archive appended by
//! `yappuccino-pack`, it installs; run with `--uninstall`, as the copy it
//! leaves in the install folder is, it uninstalls. `--quiet` does either
//! without a window, for scripts. See docs/packaging.md.
#![cfg_attr(not(test), windows_subsystem = "windows")]

// `pack` and its helpers are yappuccino-pack's; this binary only reads.
#[allow(dead_code)]
mod pack;
#[cfg(windows)]
mod system;

slint::slint! {
    #[edition = "1.19"]
    export { SetupWindow } from "ui/setup.slint";
}

#[cfg(not(windows))]
fn main() {
    eprintln!(
        "yappuccino-setup installs on Windows only; Linux has the AppImage and the Arch package."
    );
    std::process::exit(1);
}

#[cfg(windows)]
fn main() {
    use slint::ComponentHandle;
    use std::path::PathBuf;
    use std::sync::Arc;

    let args: Vec<String> = std::env::args().collect();
    let uninstall = args.iter().any(|arg| arg == "--uninstall");
    let quiet = args.iter().any(|arg| arg == "--quiet");
    let exe = std::env::current_exe().expect("knows its own path");
    let payload = if uninstall {
        None
    } else {
        pack::read(&exe).ok().flatten().map(Arc::new)
    };
    let dir: PathBuf = if uninstall {
        exe.parent().unwrap().to_path_buf()
    } else {
        system::default_dir()
    };
    let task = Task {
        uninstall,
        dir,
        exe,
        payload,
        autostart: true,
        desktop: false,
    };

    if quiet {
        std::process::exit(match task.run(|_, _| ()) {
            Ok(()) => 0,
            Err(error) => {
                eprintln!("{error}");
                1
            }
        });
    }

    let window = SetupWindow::new().expect("opens a window");
    window.set_uninstall(uninstall);
    window.set_install_dir(task.dir.to_string_lossy().as_ref().into());
    window.set_version(
        task.payload
            .as_ref()
            .map(|payload| payload.version.as_str())
            .unwrap_or("")
            .into(),
    );
    if !uninstall && task.payload.is_none() {
        window.set_stage(3);
        window.set_detail("This setup carries no Yappuccino. Make it with yappuccino-pack, as docs/packaging.md says.".into());
    }
    let task = std::cell::RefCell::new(Some(task));
    let weak = window.as_weak();
    window.on_start(move || {
        let Some(mut task) = task.borrow_mut().take() else {
            return;
        };
        let window = weak.unwrap();
        task.autostart = window.get_autostart();
        task.desktop = window.get_desktop();
        window.set_stage(1);
        window.set_progress(0.0);
        let weak = weak.clone();
        // The window keeps drawing while the files go out.
        std::thread::spawn(move || {
            let shown = weak.clone();
            let result = task.run(move |progress, detail: &str| {
                let detail = detail.to_string();
                let shown = shown.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(window) = shown.upgrade() {
                        window.set_progress(progress);
                        window.set_detail(detail.into());
                    }
                });
            });
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = weak.upgrade() {
                    match result {
                        Ok(()) => window.set_stage(2),
                        Err(error) => {
                            window.set_detail(error.to_string().into());
                            window.set_stage(3);
                        }
                    }
                }
            });
        });
    });
    let dir = window.get_install_dir().to_string();
    let weak = window.as_weak();
    window.on_launch(move || {
        let _ = system::launch(std::path::Path::new(&dir));
        let _ = weak.upgrade().map(|window| window.hide());
    });
    window.on_quit(|| {
        let _ = slint::quit_event_loop();
    });
    window.run().expect("runs its window");
}

#[cfg(windows)]
struct Task {
    uninstall: bool,
    dir: std::path::PathBuf,
    exe: std::path::PathBuf,
    payload: Option<std::sync::Arc<pack::Payload>>,
    autostart: bool,
    desktop: bool,
}

#[cfg(windows)]
impl Task {
    /// Installs or uninstalls, telling `progress` how far along, from 0 to 1, and what it's doing.
    fn run(&self, mut progress: impl FnMut(f32, &str)) -> std::io::Result<()> {
        use std::io;
        progress(0.0, "Closing Yappuccino if it's running...");
        system::close_app();
        if self.uninstall {
            // Never empty a folder that isn't Yappuccino's, such as Downloads
            // after the uninstaller was copied there.
            let named = self
                .dir
                .file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case("Yappuccino"));
            if !named && !self.dir.join("yappuccino.exe").is_file() {
                return Err(io::Error::other(format!(
                    "{} isn't a Yappuccino install folder, so nothing was removed.",
                    self.dir.display()
                )));
            }
            progress(0.3, "Removing it from Windows...");
            system::unregister(&self.dir);
            system::shortcuts(None)?;
            progress(0.6, "Removing its files...");
            system::remove_folder(&self.dir)?;
            progress(1.0, "Done");
            return Ok(());
        }
        let payload = self
            .payload
            .as_ref()
            .ok_or_else(|| io::Error::other("This setup carries no Yappuccino."))?;
        std::fs::create_dir_all(&self.dir)?;
        let total = payload.total.max(1) as f32;
        // Files take the bar to 90%; registering the rest.
        payload
            .unpack(&self.dir, |done| {
                progress(0.9 * done as f32 / total, "Copying files...")
            })
            .map_err(|error| {
                io::Error::other(format!("Couldn't copy Yappuccino's files: {error}"))
            })?;
        progress(0.9, "Adding it to Windows...");
        let mut stub = std::fs::File::open(&self.exe)?;
        let mut uninstaller = std::fs::File::create(self.dir.join(system::UNINSTALLER))?;
        io::copy(
            &mut io::Read::take(&mut stub, payload.stub_len),
            &mut uninstaller,
        )?;
        system::register(&self.dir, &payload.version, payload.total, self.autostart)?;
        system::shortcuts(Some((&self.dir.join("yappuccino.exe"), self.desktop)))?;
        progress(1.0, "Done");
        Ok(())
    }
}
