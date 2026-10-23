//! Língua dos textos que o Rust monta para a interface: inglês por padrão,
//! português quando o sistema está em português. A interface em Slint usa
//! `@tr` e as traduções de `app/translations`; aqui é o mesmo para as
//! mensagens que nascem no Rust (avisos, estado, catálogo de imagens).
//!
//! O app chama `usar_portugues` uma vez, no início, com o mesmo idioma que
//! passa ao Slint. Um texto sem tradução na tabela sai em inglês.

use std::fmt::Display;
use std::sync::atomic::{AtomicBool, Ordering};

static PORTUGUES: AtomicBool = AtomicBool::new(false);

pub fn usar_portugues(sim: bool) {
    PORTUGUES.store(sim, Ordering::Relaxed);
}

pub fn portugues() -> bool {
    PORTUGUES.load(Ordering::Relaxed)
}

/// `true` se o código de idioma do sistema (`pt-BR`, `pt_PT`, `pt`) é português.
pub fn eh_portugues(locale: &str) -> bool {
    let l = locale.to_ascii_lowercase();
    l == "pt" || l.starts_with("pt-") || l.starts_with("pt_")
}

/// Texto fixo, em inglês, traduzido se for o caso.
pub fn t(en: &'static str) -> &'static str {
    traduz(portugues(), en)
}

/// Texto com campos `{nome}`: traduz o modelo e troca os campos.
pub fn tf(en: &'static str, campos: &[(&str, &dyn Display)]) -> String {
    preenche(portugues(), en, campos)
}

/// Número com `casas` decimais e a vírgula do português quando for o caso.
pub fn num(x: f64, casas: usize) -> String {
    numero(portugues(), x, casas)
}

fn traduz(pt: bool, en: &'static str) -> &'static str {
    if !pt {
        return en;
    }
    PT_BR.iter().find(|(e, _)| *e == en).map_or(en, |(_, p)| p)
}

fn preenche(pt: bool, en: &'static str, campos: &[(&str, &dyn Display)]) -> String {
    let mut s = traduz(pt, en).to_string();
    for (k, v) in campos {
        s = s.replace(&format!("{{{k}}}"), &v.to_string());
    }
    s
}

fn numero(pt: bool, x: f64, casas: usize) -> String {
    let s = format!("{x:.casas$}");
    if pt { s.replace('.', ",") } else { s }
}

const PT_BR: &[(&str, &str)] = &[
    // Tipos de aparelho (Kind::display)
    ("Phone", "Celular"),
    ("Tablet", "Tablet"),
    ("Foldable", "Dobrável"),
    ("Watch", "Relógio"),
    ("{kind} light", "{kind} leve"),
    // Estado e progresso
    ("starting…", "ligando…"),
    ("making room…", "abrindo espaço…"),
    ("Downloading {pct}%", "Baixando {pct}%"),
    ("Verifying and extracting", "Conferindo e extraindo"),
    ("{feito} of {total}", "{feito} de {total}"),
    ("estimating time…", "calculando o tempo…"),
    ("downloading the emulator…", "baixando o emulador…"),
    ("Emulator: {s}", "Emulador: {s}"),
    ("about {n} s left", "faltam uns {n} s"),
    ("about {n} min left", "faltam uns {n} min"),
    ("about {h} h {m} min left", "faltam umas {h} h {m} min"),
    (
        "Installed · r{v} · new r{nova} ({mb})",
        "Instalada · r{v} · nova r{nova} ({mb})",
    ),
    ("Installed · r{v}", "Instalada · r{v}"),
    (
        "Downloads {mb} before creating.",
        "Baixa {mb} antes de criar.",
    ),
    (
        "Uses about {custo} GB of memory while on.",
        "Custa cerca de {custo} GB de memória ligado.",
    ),
    (
        "Computer memory: {usada} of {total} GB\nDevices {aparelhos} · swap {swap} GB · ceiling {teto} GB",
        "Memória do computador: {usada} de {total} GB\nAparelhos {aparelhos} · swap {swap} GB · teto {teto} GB",
    ),
    // Ligar
    (
        "{nome} can't start: it doesn't fit under the {teto} GB ceiling even after hibernating the others",
        "{nome} não liga: nem hibernando os outros cabe no teto de {teto} GB",
    ),
    (
        "{v} hibernated so {nome} could start",
        "{v} hibernou para {nome} ligar",
    ),
    (
        "{nome} uses {imagem}, which isn't installed",
        "{nome} usa {imagem}, que não está instalada",
    ),
    ("the emulator didn't open: {e}", "não abriu o emulador: {e}"),
    (
        "the emulator closed while starting",
        "o emulador fechou ao ligar",
    ),
    ("no gRPC connection: {e}", "sem conexão gRPC: {e}"),
    (
        "The light engine needs Docker installed",
        "O motor leve precisa do Docker instalado",
    ),
    (
        "The light engine needs binder in the kernel: sudo modprobe binder_linux devices=binder,hwbinder,vndbinder",
        "O motor leve precisa do binder no kernel: sudo modprobe binder_linux devices=binder,hwbinder,vndbinder",
    ),
    (
        "Download the platform tools (adb) in Images before starting",
        "Baixe as ferramentas da plataforma (adb) em Imagens antes de ligar",
    ),
    ("{nome} didn't start: {e}", "{nome} não ligou: {e}"),
    (
        "the video decoder didn't open",
        "o decodificador de vídeo não abriu",
    ),
    ("no Podman or Docker", "sem Podman nem Docker"),
    // Captura
    ("Screenshot saved to {pasta}", "Captura salva em {pasta}"),
    ("The screenshot wasn't saved", "A captura não foi salva"),
    // Duplicar, apagar, criar
    (
        "Shut down {nome} before duplicating",
        "Desligue {nome} antes de duplicar",
    ),
    (
        "Duplicating doesn't work for light devices yet",
        "Duplicar ainda não vale para aparelho leve",
    ),
    (
        "{novo} created from {nome}",
        "{novo} criado a partir de {nome}",
    ),
    ("Couldn't duplicate: {e}", "Não duplicou: {e}"),
    (
        "Shut down {nome} before deleting",
        "Desligue {nome} antes de apagar",
    ),
    ("{nome} was deleted", "{nome} foi apagado"),
    ("Couldn't delete: {e}", "Não apagou: {e}"),
    (
        "Use a name with letters, numbers, dot, hyphen or underscore (no spaces).",
        "Dê um nome com letras, números, ponto, hífen ou sublinhado (sem espaço).",
    ),
    (
        "A device with that name already exists.",
        "Já existe um aparelho com esse nome.",
    ),
    (
        "{nome} created on the light engine",
        "{nome} criado no motor leve",
    ),
    ("Choose an image.", "Escolha uma imagem."),
    ("{nome} created", "{nome} criado"),
    // Imagens
    (
        "Couldn't read the repository: {falhas}",
        "Não li do repositório: {falhas}",
    ),
    (
        "Shut down the devices that use this image first",
        "Desligue os aparelhos que usam esta imagem antes",
    ),
    ("Couldn't remove: {e}", "Não removeu: {e}"),
    (
        "This image isn't in the repository right now; try again in Images.",
        "Esta imagem não está no repositório agora; tente de novo em Imagens.",
    ),
    (
        "Shut down the devices that use this image before updating",
        "Desligue os aparelhos que usam esta imagem antes de atualizar",
    ),
    (
        "Couldn't record the acceptance: {e}",
        "Não gravou o aceite: {e}",
    ),
    // Catálogo (catalog.rs)
    ("Android Emulator", "Emulador do Android"),
    ("Platform tools (adb)", "Ferramentas da plataforma (adb)"),
    (
        "Android {v} with Google Play",
        "Android {v} com Google Play",
    ),
    (
        "Android {v} with Google APIs",
        "Android {v} com APIs do Google",
    ),
    (
        "Android {v} without Google (AOSP)",
        "Android {v} sem Google (AOSP)",
    ),
    (
        "Runs the devices. Required.",
        "Roda os aparelhos. Obrigatório.",
    ),
    (
        "Connects the screen and touch of light devices (Redroid). Required for the light engine.",
        "Liga a tela e o toque dos aparelhos leves (Redroid). Obrigatório para o motor leve.",
    ),
    ("Phone, tablet and foldable", "Celular, tablet e dobrável"),
    (" · with Play Store", " · com Play Store"),
    (
        " · no Play Store; apps by APK",
        " · sem Play Store; apps por APK",
    ),
    (
        " · no Google apps at all; apps by APK",
        " · sem nenhum app do Google; apps por APK",
    ),
    (
        "Recommended: the lightest, with Play Store (the emulator uses 2 GB of RAM; ~4.6 GB per phone).",
        "Recomendada: a mais leve, com Play Store (o emulador usa 2 GB de RAM; ~4,6 GB por celular).",
    ),
    (
        "Recommended: the newest that is still light, with Play Store (2.5 GB of RAM; ~5.3 GB per phone).",
        "Recomendada: a mais nova que ainda é leve, com Play Store (2,5 GB de RAM; ~5,3 GB por celular).",
    ),
    (
        "Recommended: the only Wear OS version that starts on this emulator.",
        "Recomendada: a única versão do Wear OS que liga neste emulador.",
    ),
    (
        "Didn't start on this emulator in MultiDroid's tests.",
        "Não ligou neste emulador nos testes do MultiDroid.",
    ),
    (
        "The emulator forces 4 GB of RAM on this version: ~6.5 GB per phone.",
        "O emulador força 4 GB de RAM nesta versão: ~6,5 GB por celular.",
    ),
    (
        "The emulator forces 2.5 GB of RAM on this version: ~0.7 GB more per phone than Android 13.",
        "O emulador força 2,5 GB de RAM nesta versão: ~0,7 GB a mais por celular que o Android 13.",
    ),
    // Erros do núcleo
    (
        "use only letters, numbers, dot, hyphen and underscore",
        "use só letras, números, ponto, hífen e sublinhado",
    ),
    (
        "a device with that name already exists",
        "já existe um aparelho com esse nome",
    ),
    (
        "{name} already exists in MultiDroid",
        "já existe {name} no MultiDroid",
    ),
    (
        "couldn't remove the volume: {e}",
        "não apagou o volume: {e}",
    ),
    (
        "the downloaded file doesn't match the published SHA-1",
        "o arquivo baixado não confere com o SHA-1 publicado",
    ),
    (
        "couldn't extract the file: {e}",
        "não extraiu o arquivo: {e}",
    ),
    (
        "not enough space: needs {gb} GB free",
        "falta espaço: precisa de {gb} GB livres",
    ),
    (
        "emulator (not available for this system)",
        "emulador (não há para este sistema)",
    ),
    ("the download failed: {e}", "o download falhou: {e}"),
    ("empty file", "arquivo vazio"),
    ("download cancelled", "download cancelado"),
    (
        "{serial} didn't finish booting in {timeout}",
        "{serial} não terminou o boot em {timeout}",
    ),
    (
        "the downloaded scrcpy server doesn't match the pinned SHA-256",
        "o servidor do scrcpy baixado não confere com o SHA-256 fixado",
    ),
    (
        "the scrcpy server didn't respond",
        "o servidor do scrcpy não respondeu",
    ),
    ("the destination already exists", "o destino já existe"),
    ("invalid token", "token inválido"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traduz_e_troca_campos() {
        // Sem mexer no idioma global: os outros testes rodam em paralelo, em inglês.
        assert!(eh_portugues("pt-BR") && eh_portugues("pt_PT") && eh_portugues("pt"));
        assert!(!eh_portugues("en-US") && !eh_portugues("es"));
        assert_eq!(
            preenche(false, "{nome} created", &[("nome", &"Celular1")]),
            "Celular1 created"
        );
        assert_eq!(
            preenche(true, "{nome} created", &[("nome", &"Celular1")]),
            "Celular1 criado"
        );
        assert_eq!(traduz(true, "Watch"), "Relógio");
        assert_eq!(traduz(true, "texto sem tradução"), "texto sem tradução");
        assert_eq!(numero(false, 4.56, 1), "4.6");
        assert_eq!(numero(true, 4.56, 1), "4,6");
    }

    #[test]
    fn tabela_sem_chave_repetida() {
        let mut vistos = std::collections::HashSet::new();
        for (en, _) in PT_BR {
            assert!(vistos.insert(en), "repetida: {en}");
        }
    }
}
