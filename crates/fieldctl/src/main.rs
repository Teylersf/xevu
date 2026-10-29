use fieldctl as xevuctl;
use std::{env, fs, path::Path};
use zeroize::Zeroizing;
fn password() -> xevuctl::Result<Zeroizing<String>> {
    Ok(Zeroizing::new(rpassword::prompt_password(
        "Vault passphrase (never saved): ",
    )?))
}
fn run() -> xevuctl::Result<()> {
    let a: Vec<String> = env::args().collect();
    if a.len() < 3 {
        println!(
            "XEVU / XEVUCTL\n\n  xevuctl init VAULT_DIRECTORY\n  xevuctl add VAULT_DIRECTORY FILE [FILE ...]\n  xevuctl list VAULT_DIRECTORY\n  xevuctl verify VAULT_DIRECTORY\n  xevuctl extract VAULT_DIRECTORY ENCRYPTED_FILENAME OUTPUT_PATH\n\nPassphrases are entered privately, never as command-line arguments.\nThis is experimental software, not approved for classified material."
        );
        return Ok(());
    }
    let root = Path::new(&a[2]);
    let p = password()?;
    if a[1] == "init" {
        let confirm = Zeroizing::new(rpassword::prompt_password(
            "Repeat passphrase (at least 16 bytes): ",
        )?);
        if *p != *confirm {
            return Err("passphrases differ".into());
        }
        xevuctl::init(root, p.as_bytes())?;
        println!("Vault created. Keep the passphrase separately from the drive.");
        return Ok(());
    }
    let key = xevuctl::unlock(root, p.as_bytes())?;
    drop(p);
    match a[1].as_str() {
        "add" => {
            if a.len() < 4 {
                return Err("supply at least one input file".into());
            }
            for file in &a[3..] {
                let id = xevuctl::add(root, &key, Path::new(file))?;
                println!("Imported and authenticated: {id}.NGV");
            }
        }
        "list" | "verify" => {
            let mut paths: Vec<_> = fs::read_dir(root)?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("NGV")))
                .collect();
            paths.sort();
            for path in paths {
                let (name, n) = if a[1] == "verify" {
                    xevuctl::verify(&path, &key)?
                } else {
                    let (_, c) = xevuctl::inspect(&path, &key)?;
                    (c.name.clone(), c.length)
                };
                println!(
                    "{}  {:>12}  {}",
                    path.file_name().unwrap().to_string_lossy(),
                    n,
                    name
                );
            }
        }
        "extract" => {
            if a.len() != 5 {
                return Err("supply encrypted filename and explicit output path".into());
            }
            let name = Path::new(&a[3]);
            if name.components().count() != 1 {
                return Err("encrypted filename must be a basename".into());
            }
            xevuctl::extract(&root.join(name), &key, Path::new(&a[4]))?;
            println!("Authenticated file exported to the explicit plaintext destination.");
        }
        _ => return Err("unknown command".into()),
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("XEVU: {e}");
        std::process::exit(1);
    }
}
