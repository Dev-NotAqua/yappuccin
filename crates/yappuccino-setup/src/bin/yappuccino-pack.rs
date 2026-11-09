//! Makes the installer: `yappuccino-pack <setup stub> <package folder> <out.exe> <version>`,
//! where the stub is yappuccino-setup.exe as built and the folder holds the
//! portable ZIP's files.

#[path = "../pack.rs"]
#[allow(dead_code)]
mod pack;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, stub, folder, out, version] = args.as_slice() else {
        eprintln!(
            "usage: yappuccino-pack <yappuccino-setup.exe> <package folder> <out.exe> <version>"
        );
        std::process::exit(2);
    };
    let stub_bytes =
        std::fs::read(stub).unwrap_or_else(|error| fail(&format!("reading {stub}: {error}")));
    if pack::read(std::path::Path::new(stub))
        .ok()
        .flatten()
        .is_some()
    {
        fail(&format!(
            "{stub} already carries a payload; give the setup program as built"
        ));
    }
    let payload = pack::pack(std::path::Path::new(folder), version)
        .unwrap_or_else(|error| fail(&format!("packing {folder}: {error}")));
    pack::write_setup(&stub_bytes, &payload, std::path::Path::new(out))
        .unwrap_or_else(|error| fail(&format!("writing {out}: {error}")));
    println!(
        "{out}: {} MB",
        (stub_bytes.len() + payload.len()) / 1_000_000
    );
}

fn fail(message: &str) -> ! {
    eprintln!("yappuccino-pack: {message}");
    std::process::exit(1);
}
