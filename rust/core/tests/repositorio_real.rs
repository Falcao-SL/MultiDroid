//! Testes contra o repositório oficial, de verdade. Não rodam por padrão (precisam de
//! rede e baixam centenas de MB); rode com `cargo test -- --ignored`.

use multidroid_core::catalog::{self, EMULATOR, Host, REPO};

fn get(url: &str) -> String {
    ureq::get(url)
        .call()
        .expect("sem rede")
        .into_string()
        .unwrap()
}

fn base(url: &str) -> String {
    url.rsplit_once('/').unwrap().0.to_string() + "/"
}

#[test]
#[ignore]
fn listas_reais_tem_as_imagens_padrao_da_0_2() {
    let mac = Host::MAC_ARM;
    let url = catalog::tag_list_url("android-wear");
    let p = catalog::parse(
        &get(&url),
        "system-images;android-33;android-wear;arm64-v8a",
        &base(&url),
        mac,
    )
    .unwrap()
    .unwrap();
    assert!(p.url.ends_with(".zip") && p.sha1.len() == 40 && !p.license_text.is_empty());
    let repo = get(&format!("{REPO}repository2-3.xml"));
    for host in [Host::MAC_ARM, Host::WINDOWS_X64, Host::LINUX_X64] {
        assert!(
            catalog::parse(&repo, EMULATOR, REPO, host)
                .unwrap()
                .is_some(),
            "{host:?} sem emulador"
        );
    }
}

#[test]
#[ignore]
fn instala_a_imagem_aosp_33_de_verdade() {
    let mac = Host::MAC_ARM;
    let url = catalog::tag_list_url("default");
    let path = format!("system-images;android-33;default;{}", mac.abi);
    let pkg = catalog::parse(&get(&url), &path, &base(&url), mac)
        .unwrap()
        .unwrap();
    let t = std::env::temp_dir().join("multidroid-teste-real");
    let _ = std::fs::remove_dir_all(&t);
    let dst = multidroid_core::install::install(&pkg, &t.join("sdk"), &t, |_| {}).unwrap();
    assert!(dst.join("system.img").is_file());
    assert!(catalog::installed(&path, &t.join("sdk")).is_some());
    let _ = std::fs::remove_dir_all(&t);
}
