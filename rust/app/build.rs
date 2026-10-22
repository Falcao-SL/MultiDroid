fn main() {
    // O inglês é a língua principal; o português vem de translations/pt_BR,
    // embutido no binário e escolhido pelo idioma do sistema no main.rs.
    let config = slint_build::CompilerConfiguration::new().with_bundled_translations("translations");
    slint_build::compile_with_config("ui/app.slint", config).expect("ui/app.slint não compilou");

    // Ícone do .exe no Explorer e na barra de tarefas do Windows.
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("icons/icon.ico");
        res.compile().expect("ícone do Windows não compilou");
    }
}
