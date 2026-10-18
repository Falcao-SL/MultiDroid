//! O estado salvo em <dados>/state.json: ordem, grupos, teto de memória, mesas e
//! o lugar de cada aparelho na grade livre. Mesmo formato do `AppModel.Saved` da
//! 0.2, para as duas versões abrirem o mesmo arquivo. Campos que esta versão não
//! conhece são guardados e regravados como vieram.

use crate::devices::{Device, Kind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub const MAIN_DESK: &str = "main";

/// Arranjo de uma mesa. Os valores gravados são os da 0.2.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Layout {
    #[default]
    #[serde(rename = "Grade")]
    Grade,
    #[serde(rename = "Automática")]
    Auto,
    #[serde(rename = "Por grupo")]
    Grupos,
}

/// Lugar de um aparelho na grade livre, em células.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spot {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Spot {
    pub fn intersects(&self, o: &Spot) -> bool {
        self.x < o.x + o.w && o.x < self.x + self.w && self.y < o.y + o.h && o.y < self.y + self.h
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub order: Vec<String>,
    /// aparelho → grupo
    pub groups: BTreeMap<String, String>,
    #[serde(rename = "ceilingGB", serialize_with = "whole_as_int")]
    pub ceiling_gb: f64,
    /// aparelho → lugar na grade livre
    #[serde(default)]
    pub spots: BTreeMap<String, Spot>,
    /// Arranjo antigo, de antes das várias mesas; lido e convertido em `layouts`.
    #[serde(default, skip_serializing)]
    layout: Option<Layout>,
    /// mesa → arranjo
    #[serde(default)]
    pub layouts: BTreeMap<String, Layout>,
    /// aparelho → mesa (sem entrada = principal)
    #[serde(default)]
    pub workspace_of: BTreeMap<String, String>,
    /// mesas, na ordem de criação
    #[serde(default = "main_only")]
    pub workspaces: Vec<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// 18.0 sai como 18, como o JSONEncoder do Swift grava.
fn whole_as_int<S: serde::Serializer>(v: &f64, s: S) -> Result<S::Ok, S::Error> {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        s.serialize_i64(*v as i64)
    } else {
        s.serialize_f64(*v)
    }
}

fn main_only() -> Vec<String> {
    vec![MAIN_DESK.into()]
}

impl State {
    /// Estado de quem nunca abriu o app: teto em 75% da memória do computador.
    pub fn new(physical_gb: f64) -> State {
        State {
            order: vec![],
            groups: BTreeMap::new(),
            ceiling_gb: (physical_gb * 0.75).round(),
            spots: BTreeMap::new(),
            layout: None,
            layouts: BTreeMap::from([(MAIN_DESK.into(), Layout::Grade)]),
            workspace_of: BTreeMap::new(),
            workspaces: main_only(),
            extra: BTreeMap::new(),
        }
    }

    pub fn parse(json: &str) -> serde_json::Result<State> {
        let mut s: State = serde_json::from_str(json)?;
        if s.layouts.is_empty() {
            s.layouts
                .insert(MAIN_DESK.into(), s.layout.take().unwrap_or_default());
        }
        if s.workspaces.is_empty() {
            s.workspaces = main_only();
        }
        Ok(s)
    }

    /// Arquivo que falta ou não abre vira o estado inicial, como na 0.2.
    pub fn load(path: &Path, physical_gb: f64) -> State {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|t| State::parse(&t).ok())
            .unwrap_or_else(|| State::new(physical_gb))
    }

    /// Grava num arquivo temporário e troca, para uma queda no meio não deixar o JSON pela metade.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec(self)?)?;
        std::fs::rename(tmp, path)
    }

    pub fn desk_of(&self, name: &str) -> &str {
        self.workspace_of.get(name).map_or(MAIN_DESK, |s| s)
    }

    pub fn layout_of(&self, desk: &str) -> Layout {
        self.layouts.get(desk).copied().unwrap_or_default()
    }

    /// Aparelhos na ordem da grade: primeiro os de `order`, depois os novos por nome.
    pub fn ordered<'a>(&self, devices: &'a [Device]) -> Vec<&'a Device> {
        let mut out: Vec<&Device> = self
            .order
            .iter()
            .filter_map(|n| devices.iter().find(|d| &d.name == n))
            .collect();
        let mut rest: Vec<&Device> = devices
            .iter()
            .filter(|d| !self.order.contains(&d.name))
            .collect();
        rest.sort_by(|a, b| a.name.cmp(&b.name));
        out.extend(rest);
        out
    }

    /// Lugar de cada aparelho da lista; quem não tem ganha o primeiro livre entre
    /// os da mesma lista (a mesma mesa). Diz se mudou algo.
    pub fn ensure_spots(&mut self, list: &[&Device]) -> bool {
        let mut changed = false;
        for d in list {
            if self.spots.contains_key(&d.name) {
                continue;
            }
            let (w, h) = d.kind.grid_size();
            let taken: Vec<Spot> = list
                .iter()
                .filter_map(|o| self.spots.get(&o.name).copied())
                .collect();
            self.spots
                .insert(d.name.clone(), grid::first_free(w, h, &taken));
            changed = true;
        }
        changed
    }

    /// Volta os aparelhos da lista ao tamanho padrão, um ao lado do outro.
    pub fn reset_spots(&mut self, list: &[&Device]) {
        let mut placed: Vec<Spot> = vec![];
        for d in list {
            let (w, h) = d.kind.grid_size();
            let s = grid::first_free(w, h, &placed);
            placed.push(s);
            self.spots.insert(d.name.clone(), s);
        }
    }

    /// Um aparelho novo ou duplicado herda grupo e mesa.
    pub fn copy_from(&mut self, from: &str, to: &str) {
        if let Some(g) = self.groups.get(from).cloned() {
            self.groups.insert(to.into(), g);
        }
        if let Some(w) = self.workspace_of.get(from).cloned() {
            self.workspace_of.insert(to.into(), w);
        }
    }

    /// Esquece um aparelho apagado.
    pub fn forget(&mut self, name: &str) {
        self.groups.remove(name);
        self.spots.remove(name);
        self.workspace_of.remove(name);
        self.order.retain(|n| n != name);
    }
}

/// A grade livre (MAC.md §2.6): 24 colunas de células quadradas.
pub mod grid {
    use super::Spot;
    use std::collections::BTreeMap;

    pub const COLS: i32 = 24;
    pub const MIN_W: i32 = 4;
    pub const MIN_H: i32 = 5;

    /// Primeiro lugar livre, linha a linha, da esquerda para a direita.
    pub fn first_free(w: i32, h: i32, taken: &[Spot]) -> Spot {
        let w = w.clamp(1, COLS);
        for y in 0.. {
            for x in 0..=(COLS - w) {
                let s = Spot { x, y, w, h };
                if !taken.iter().any(|t| t.intersects(&s)) {
                    return s;
                }
            }
        }
        unreachable!()
    }

    /// Põe `id` em `spot` e empurra para baixo, em cascata, quem colidir.
    pub fn place(id: &str, spot: Spot, spots: &BTreeMap<String, Spot>) -> BTreeMap<String, Spot> {
        let mut out = spots.clone();
        out.insert(id.into(), spot);
        let mut queue = vec![id.to_string()];
        while let Some(a) = queue.pop() {
            let Some(sa) = out.get(&a).copied() else {
                continue;
            };
            let hits: Vec<String> = out
                .iter()
                .filter(|(b, sb)| **b != a && *b != id && sb.intersects(&sa))
                .map(|(b, _)| b.clone())
                .collect();
            for b in hits {
                if let Some(sb) = out.get_mut(&b) {
                    sb.y = sa.y + sa.h;
                }
                queue.push(b);
            }
        }
        out
    }
}

/// Memória estimada dos aparelhos ligados, para comparar com o teto.
pub fn estimate_gb(kinds: impl IntoIterator<Item = Kind>) -> f64 {
    kinds.into_iter().map(Kind::estimate_gb).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// O state.json que a 0.2 gravou no Mac do dono (só nomes de aparelho e números).
    const DA_02: &str = r#"{"workspaces":["main"],"order":["Personal","Shop","Studio","Watch"],"groups":{},"layouts":{"main":"Grade"},"ceilingGB":18,"spots":{"Shop":{"y":6,"h":8,"x":7,"w":7},"Watch":{"y":0,"h":6,"x":7,"w":7},"Personal":{"y":0,"h":14,"x":0,"w":7},"Studio":{"y":0,"h":14,"x":14,"w":7}},"workspaceOf":{}}"#;

    fn dev(name: &str, kind: Kind) -> Device {
        Device {
            name: name.into(),
            kind,
            api: "33".into(),
            screen: String::new(),
            dir: PathBuf::new(),
            image: String::new(),
        }
    }

    #[test]
    fn le_o_estado_da_02_e_regrava_igual() {
        let s = State::parse(DA_02).unwrap();
        assert_eq!(s.order, ["Personal", "Shop", "Studio", "Watch"]);
        assert_eq!(s.ceiling_gb, 18.0);
        assert_eq!(s.layout_of(MAIN_DESK), Layout::Grade);
        assert_eq!(
            s.spots["Shop"],
            Spot {
                x: 7,
                y: 6,
                w: 7,
                h: 8
            }
        );
        let back: serde_json::Value = serde_json::to_value(&s).unwrap();
        let orig: serde_json::Value = serde_json::from_str(DA_02).unwrap();
        assert_eq!(back, orig);
    }

    #[test]
    fn campos_desconhecidos_e_formato_antigo() {
        let s = State::parse(
            r#"{"order":[],"groups":{"A":"Loja"},"ceilingGB":12,"layout":"Por grupo","novo":{"x":1}}"#,
        )
        .unwrap();
        assert_eq!(s.layout_of(MAIN_DESK), Layout::Grupos);
        assert_eq!(s.workspaces, [MAIN_DESK]);
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["novo"]["x"], 1);
        assert!(v.get("layout").is_none());
        assert_eq!(v["layouts"]["main"], "Por grupo");
    }

    #[test]
    fn grava_e_le_de_volta() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("state.json");
        assert_eq!(State::load(&p, 24.0).ceiling_gb, 18.0);
        let mut s = State::parse(DA_02).unwrap();
        s.groups.insert("Shop".into(), "Lojas".into());
        s.save(&p).unwrap();
        assert_eq!(State::load(&p, 24.0), s);
        std::fs::write(&p, "{quebrado").unwrap();
        assert!(State::load(&p, 24.0).order.is_empty());
    }

    #[test]
    fn ordem_e_lugares() {
        let mut s = State::parse(DA_02).unwrap();
        let devs = vec![
            dev("Watch", Kind::Relogio),
            dev("Novo", Kind::Tablet),
            dev("Personal", Kind::Celular),
        ];
        let o = s.ordered(&devs);
        assert_eq!(
            o.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(),
            ["Personal", "Watch", "Novo"]
        );
        assert!(s.ensure_spots(&o));
        let novo = s.spots["Novo"];
        assert_eq!((novo.w, novo.h), (12, 8));
        assert!(!novo.intersects(&s.spots["Personal"]) && !novo.intersects(&s.spots["Watch"]));
        assert!(!s.ensure_spots(&o));
        s.forget("Novo");
        assert!(!s.spots.contains_key("Novo"));
    }

    #[test]
    fn soltar_em_cima_empurra_para_baixo() {
        let mut spots = BTreeMap::new();
        spots.insert(
            "A".to_string(),
            Spot {
                x: 0,
                y: 0,
                w: 7,
                h: 14,
            },
        );
        spots.insert(
            "B".to_string(),
            Spot {
                x: 0,
                y: 14,
                w: 7,
                h: 6,
            },
        );
        let out = grid::place(
            "C",
            Spot {
                x: 0,
                y: 0,
                w: 7,
                h: 4,
            },
            &spots,
        );
        assert_eq!(out["A"].y, 4);
        assert_eq!(out["B"].y, 18);
        assert_eq!(grid::first_free(30, 2, &[]).w, grid::COLS);
    }
}
