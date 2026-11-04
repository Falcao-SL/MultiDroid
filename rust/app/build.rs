fn main() {
    // Tag completa da release (ex. 0.3.0-alpha.4) no pacote do CI; localmente
    // cai na versão do Cargo.toml. Sem isso, alpha.N não se compara com alpha.N+1.
    let release = std::env::var("MULTIDROID_RELEASE")
        .or_else(|_| std::env::var("VERSION"))
        .unwrap_or_else(|_| std::env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION"));
    println!("cargo:rustc-env=MULTIDROID_RELEASE={release}");
    println!("cargo:rerun-if-env-changed=MULTIDROID_RELEASE");
    println!("cargo:rerun-if-env-changed=VERSION");

    // O inglês é a língua principal; o português vem de translations/pt_BR,
    // embutido no binário e escolhido pelo idioma do sistema no main.rs.
    let config =
        slint_build::CompilerConfiguration::new().with_bundled_translations("translations");
    slint_build::compile_with_config("ui/app.slint", config).expect("ui/app.slint não compilou");

    // Ícone do .exe no Explorer e na barra de tarefas do Windows.
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("icons/icon.ico");
        res.compile().expect("ícone do Windows não compilou");
    }
}
