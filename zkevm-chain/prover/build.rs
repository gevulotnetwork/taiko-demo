use std::env::var;
use std::process::Command;

fn run(cmd: &str, args: Vec<&str>) -> String {
    let err = format!("Error running: {} {:#?}", cmd, &args);
    let result = Command::new(cmd).args(&args).output().expect(&err);
    println!("{:?}", std::str::from_utf8(&result.stderr));
    assert!(result.status.success(), "Command failed: {err}");

    String::from_utf8(result.stdout).expect("utf8 output")
}

fn get_crate_version(pkg: &str) -> String {
    run(
        "cargo",
        vec![
            "tree",
            "--package",
            pkg,
            "--depth",
            "0",
            "--prefix",
            "none",
            "--quiet",
            "--charset",
            "utf8",
        ],
    )
}

fn main() {
    let pkg_version = var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION");
    let version = format!(
        "{} {} {}",
        pkg_version,
        run(
            "git",
            vec![
                "-c",
                "safe.directory=*",
                "describe",
                "--all",
                "--long",
                "--dirty"
            ]
        ),
        get_crate_version("zkevm-circuits"),
    );
    println!(
        "cargo:rustc-env=PROVER_VERSION={}",
        version.replace('\n', "")
    );

    println!("cargo:rustc-link-lib=static=solc");
    println!("cargo:rustc-link-lib=static=solidity");
    println!("cargo:rustc-link-lib=static=evmasm");
    println!("cargo:rustc-link-lib=static=smtutil");
    println!("cargo:rustc-link-lib=static=yul");
    println!("cargo:rustc-link-lib=static=yulInterpreter");
    println!("cargo:rustc-link-lib=static=langutil");
    println!("cargo:rustc-link-lib=static=solutil");
    println!("cargo:rustc-link-lib=static=jsoncpp");
    println!("cargo:rustc-link-lib=static=stdc++");
    println!("cargo:rustc-link-lib=static=gcc");
    println!("cargo:rustc-link-lib=static=boost_filesystem");
    println!("cargo:rustc-link-search=native=/usr/lib/gcc/x86_64-linux-gnu/11");
    println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu");
    println!("cargo:rustc-link-search=/home/ader/dev/eiger/gev/kyle-zkevm-chain/lib");
}
