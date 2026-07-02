use crate::diagnostics::Diagnostic;
use crate::parser::DatFile;
use std::path::Path;

/// 1つの `Rule` が検査を行うために必要な入力一式。
pub struct RuleContext<'a> {
    pub dat: &'a DatFile,
    pub dat_dir: &'a Path,
}

/// 1つのobj種別に対する1つの検査項目。obj種別ごとの`Vec<Box<dyn Rule>>`を
/// `RuleSet`にまとめ、`obj=`の値で選択する。対応obj種別が2つ程度の現段階では
/// マクロによる自動登録は過剰設計と判断し、各obj種別モジュールの`all()`関数が
/// 素朴にVecを組み立てる方式を採る。
pub trait Rule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic>;
}

/// このプロジェクトが検証可能な`obj=`の値一覧（単一の正）。`for_obj_type`の
/// match armと`formatter::order::order_for`のmatch armは、ここに列挙した22種別と
/// 同じ集合・同じ順序を保つこと（`main.rs`のヘルプ文言・エラーメッセージ、および
/// `tests/obj_type_coverage.rs`はこのリストを参照して整合性を検証する）。
pub const SUPPORTED_OBJ_TYPES: &[&str] = &[
    "building",
    "vehicle",
    "way",
    "good",
    "bridge",
    "tunnel",
    "roadsign",
    "crossing",
    "way-object",
    "ground_obj",
    "tree",
    "citycar",
    "pedestrian",
    "factory",
    "sound",
    "ground",
    "menu",
    "cursor",
    "symbol",
    "smoke",
    "field",
    "misc",
];

pub struct RuleSet {
    rules: Vec<Box<dyn Rule>>,
}

impl RuleSet {
    pub fn new(rules: Vec<Box<dyn Rule>>) -> Self {
        RuleSet { rules }
    }

    /// `obj=`の値に応じたルール集合を返す。未対応のobj種別には`None`。
    pub fn for_obj_type(obj: &str, dat: &DatFile) -> Option<RuleSet> {
        match obj {
            "building" => Some(RuleSet::new(crate::rules::building::all(dat))),
            "vehicle" => Some(RuleSet::new(crate::rules::vehicle::all())),
            "way" => Some(RuleSet::new(crate::rules::way::all())),
            "good" => Some(RuleSet::new(crate::rules::good::all())),
            "bridge" => Some(RuleSet::new(crate::rules::bridge::all())),
            "tunnel" => Some(RuleSet::new(crate::rules::tunnel::all())),
            "roadsign" => Some(RuleSet::new(crate::rules::roadsign::all())),
            "crossing" => Some(RuleSet::new(crate::rules::crossing::all())),
            "way-object" => Some(RuleSet::new(crate::rules::way_obj::all())),
            "ground_obj" => Some(RuleSet::new(crate::rules::groundobj::all())),
            "tree" => Some(RuleSet::new(crate::rules::tree::all())),
            "citycar" => Some(RuleSet::new(crate::rules::citycar::all())),
            "pedestrian" => Some(RuleSet::new(crate::rules::pedestrian::all())),
            "factory" => Some(RuleSet::new(crate::rules::factory::all(dat))),
            "sound" => Some(RuleSet::new(crate::rules::sound::all())),
            "ground" => Some(RuleSet::new(crate::rules::ground::all())),
            "menu" => Some(RuleSet::new(crate::rules::menu::all())),
            "cursor" => Some(RuleSet::new(crate::rules::cursor::all())),
            "symbol" => Some(RuleSet::new(crate::rules::symbol::all())),
            "smoke" => Some(RuleSet::new(crate::rules::smoke::all())),
            "field" => Some(RuleSet::new(crate::rules::field::all())),
            "misc" => Some(RuleSet::new(crate::rules::misc::all())),
            _ => None,
        }
    }

    pub fn run(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        self.rules.iter().flat_map(|r| r.check(ctx)).collect()
    }
}
