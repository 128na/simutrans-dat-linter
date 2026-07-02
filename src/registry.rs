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

/// このプロジェクトが検証可能な`obj=`の値（単一の正）。`for_obj_type`と
/// `formatter::order::order_for`はどちらもこの列挙型を経由してディスパッチするため、
/// 23種別目を追加する際は`from_str`・`as_str`・両モジュールのmatch armの4箇所全てを
/// 更新しないと`cargo build`が非網羅match errorで失敗する（コンパイル時に強制される。
/// 詳細は`for_obj_type`・`formatter::order::order_for`のdocコメント参照）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjType {
    Building,
    Vehicle,
    Way,
    Good,
    Bridge,
    Tunnel,
    Roadsign,
    Crossing,
    WayObject,
    GroundObj,
    Tree,
    Citycar,
    Pedestrian,
    Factory,
    Sound,
    Ground,
    Menu,
    Cursor,
    Symbol,
    Smoke,
    Field,
    Misc,
}

impl ObjType {
    /// `obj=`の値（文字列）からの変換。未知の文字列には`None`。
    ///
    /// 名前は`std::str::FromStr`トレイトと紛らわしいが、要求仕様上この関数名・
    /// シグネチャ（inherent method、`&str` -> `Option<ObjType>`）を固定するため
    /// `clippy::should_implement_trait`を明示的に許容する。
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<ObjType> {
        match s {
            "building" => Some(ObjType::Building),
            "vehicle" => Some(ObjType::Vehicle),
            "way" => Some(ObjType::Way),
            "good" => Some(ObjType::Good),
            "bridge" => Some(ObjType::Bridge),
            "tunnel" => Some(ObjType::Tunnel),
            "roadsign" => Some(ObjType::Roadsign),
            "crossing" => Some(ObjType::Crossing),
            "way-object" => Some(ObjType::WayObject),
            "ground_obj" => Some(ObjType::GroundObj),
            "tree" => Some(ObjType::Tree),
            "citycar" => Some(ObjType::Citycar),
            "pedestrian" => Some(ObjType::Pedestrian),
            "factory" => Some(ObjType::Factory),
            "sound" => Some(ObjType::Sound),
            "ground" => Some(ObjType::Ground),
            "menu" => Some(ObjType::Menu),
            "cursor" => Some(ObjType::Cursor),
            "symbol" => Some(ObjType::Symbol),
            "smoke" => Some(ObjType::Smoke),
            "field" => Some(ObjType::Field),
            "misc" => Some(ObjType::Misc),
            _ => None,
        }
    }

    /// `from_str`の逆変換。**ワイルドカードarmを持たない網羅matchであること**が
    /// このリファクタの要点: 23番目のvariantを追加してここを更新し忘れると
    /// `cargo build`が失敗する。
    pub fn as_str(&self) -> &'static str {
        match self {
            ObjType::Building => "building",
            ObjType::Vehicle => "vehicle",
            ObjType::Way => "way",
            ObjType::Good => "good",
            ObjType::Bridge => "bridge",
            ObjType::Tunnel => "tunnel",
            ObjType::Roadsign => "roadsign",
            ObjType::Crossing => "crossing",
            ObjType::WayObject => "way-object",
            ObjType::GroundObj => "ground_obj",
            ObjType::Tree => "tree",
            ObjType::Citycar => "citycar",
            ObjType::Pedestrian => "pedestrian",
            ObjType::Factory => "factory",
            ObjType::Sound => "sound",
            ObjType::Ground => "ground",
            ObjType::Menu => "menu",
            ObjType::Cursor => "cursor",
            ObjType::Symbol => "symbol",
            ObjType::Smoke => "smoke",
            ObjType::Field => "field",
            ObjType::Misc => "misc",
        }
    }
}

/// このプロジェクトが検証可能な`obj=`の値一覧（`ObjType`から導出）。
/// `main.rs`のヘルプ文言・エラーメッセージはこのリストを参照する。
/// 個々の文字列と`ObjType`の対応が保たれていることは`tests/obj_type_coverage.rs`が
/// 検証する。
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
    ///
    /// 内部では`obj`文字列をまず`ObjType`にパースし、`ObjType`に対する
    /// **ワイルドカードarmを持たない網羅match**でディスパッチする。これにより
    /// `ObjType`に23番目のvariantを追加したのにこのmatchへのarm追加を忘れると
    /// `cargo build`が非網羅match errorで失敗する（`formatter::order::order_for`と
    /// 対になる、このリファクタの要点）。公開シグネチャ（`&str`入力・
    /// `Option<RuleSet>`出力）は変更しない。
    pub fn for_obj_type(obj: &str, dat: &DatFile) -> Option<RuleSet> {
        let obj_type = ObjType::from_str(obj)?;
        match obj_type {
            ObjType::Building => Some(RuleSet::new(crate::rules::building::all(dat))),
            ObjType::Vehicle => Some(RuleSet::new(crate::rules::vehicle::all())),
            ObjType::Way => Some(RuleSet::new(crate::rules::way::all())),
            ObjType::Good => Some(RuleSet::new(crate::rules::good::all())),
            ObjType::Bridge => Some(RuleSet::new(crate::rules::bridge::all())),
            ObjType::Tunnel => Some(RuleSet::new(crate::rules::tunnel::all())),
            ObjType::Roadsign => Some(RuleSet::new(crate::rules::roadsign::all())),
            ObjType::Crossing => Some(RuleSet::new(crate::rules::crossing::all())),
            ObjType::WayObject => Some(RuleSet::new(crate::rules::way_obj::all())),
            ObjType::GroundObj => Some(RuleSet::new(crate::rules::groundobj::all())),
            ObjType::Tree => Some(RuleSet::new(crate::rules::tree::all())),
            ObjType::Citycar => Some(RuleSet::new(crate::rules::citycar::all())),
            ObjType::Pedestrian => Some(RuleSet::new(crate::rules::pedestrian::all())),
            ObjType::Factory => Some(RuleSet::new(crate::rules::factory::all(dat))),
            ObjType::Sound => Some(RuleSet::new(crate::rules::sound::all())),
            ObjType::Ground => Some(RuleSet::new(crate::rules::ground::all())),
            ObjType::Menu => Some(RuleSet::new(crate::rules::menu::all())),
            ObjType::Cursor => Some(RuleSet::new(crate::rules::cursor::all())),
            ObjType::Symbol => Some(RuleSet::new(crate::rules::symbol::all())),
            ObjType::Smoke => Some(RuleSet::new(crate::rules::smoke::all())),
            ObjType::Field => Some(RuleSet::new(crate::rules::field::all())),
            ObjType::Misc => Some(RuleSet::new(crate::rules::misc::all())),
        }
    }

    pub fn run(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        self.rules.iter().flat_map(|r| r.check(ctx)).collect()
    }
}
