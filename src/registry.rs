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
            _ => None,
        }
    }

    pub fn run(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        self.rules.iter().flat_map(|r| r.check(ctx)).collect()
    }
}
