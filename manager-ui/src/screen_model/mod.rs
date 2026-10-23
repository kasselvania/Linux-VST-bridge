//! Screen model: what each manager screen says and offers.
//!
//! Plain Rust over the manager's typed operator view. It decides which facts,
//! status and offered actions a screen shows, and words them for a musician.
//! It names no UI toolkit: the window draws these structs, and another
//! frontend can draw the same ones. The manager stays the only owner of facts
//! and actions; an offer carries the manager's exact closed action and refusal
//! unchanged, and pressing it sends only that action.
//!
//! Screens built here so far: Home, Setup (with install progress, required
//! apps, existing setups and their runtime choice), the Setup forms, the
//! plug-in list and one plug-in's detail.
pub mod forms;
pub mod home;
pub mod installer;
pub mod journey;
pub mod offers;
pub mod plugins;
pub mod product;
pub mod service;
pub mod settings;
pub mod setup;
#[cfg(test)]
mod tests;

use crate::model::Product;

/// What a line of words is for. Each frontend chooses the look.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextRole {
    /// An ordinary sentence.
    Body,
    /// A short heading inside a section.
    Strong,
    /// Secondary detail, printed smaller.
    Detail,
    /// Something to notice: a problem, refusal or risk.
    Caution,
    /// A retained hazard inside a technical record.
    Alert,
    /// A quiet aside, printed smaller and fainter than detail.
    Hint,
    /// An aside that says something is not current, printed small.
    CautionHint,
}

/// One line of words with its role.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Text {
    pub role: TextRole,
    pub words: String,
}

impl Text {
    pub fn body(words: impl Into<String>) -> Self { Self { role: TextRole::Body, words: words.into() } }
    pub fn strong(words: impl Into<String>) -> Self { Self { role: TextRole::Strong, words: words.into() } }
    pub fn detail(words: impl Into<String>) -> Self { Self { role: TextRole::Detail, words: words.into() } }
    pub fn caution(words: impl Into<String>) -> Self { Self { role: TextRole::Caution, words: words.into() } }
    pub fn alert(words: impl Into<String>) -> Self { Self { role: TextRole::Alert, words: words.into() } }
    pub fn hint(words: impl Into<String>) -> Self { Self { role: TextRole::Hint, words: words.into() } }
    pub fn caution_hint(words: impl Into<String>) -> Self { Self { role: TextRole::CautionHint, words: words.into() } }
}

/// Visual class of a status: how it should look, not what it says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusClass {
    Ready,
    Working,
    NeedsYou,
    Problem,
    Unknown,
}

impl StatusClass {
    /// Problems and unknown states ask for the musician's attention.
    pub fn needs_attention(self) -> bool {
        matches!(self, Self::Problem | Self::Unknown)
    }
}

/// A status in the owner's words, with its visual class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    pub words: &'static str,
    pub class: StatusClass,
}

/// One labelled fact. A missing value is said explicitly, never left blank.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fact {
    pub label: &'static str,
    pub value: Option<String>,
    pub missing: &'static str,
}

impl Fact {
    /// The value, or the words that say it is missing.
    pub fn shown(&self) -> &str {
        self.value.as_deref().unwrap_or(self.missing)
    }
}

/// Facts that read as one line.
pub fn fact_line(facts: &[Fact]) -> String {
    facts.iter().map(Fact::shown).collect::<Vec<_>>().join(" · ")
}

/// Exact identity of one installed plug-in build in one setup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductKey {
    pub environment: String,
    pub class_id: String,
    pub module_sha256: String,
}

impl From<&Product> for ProductKey {
    fn from(product: &Product) -> Self {
        Self {
            environment: product.environment.clone(),
            class_id: product.class_id.clone(),
            module_sha256: product.module_sha256.clone(),
        }
    }
}

/// Labelled detail: words, offers and setup choices grouped under disclosures.
#[derive(Clone, Debug, PartialEq)]
pub enum Detail {
    Text(Text),
    Offers(offers::OfferRow),
    /// Offers that wait for current status: shown, but not pressable, until
    /// the screen is `actionable` again. Each offer is also `Held`, and the
    /// screen says why once.
    Held { actionable: bool, row: offers::OfferRow },
    Choice(Box<offers::SetupChoice>),
    Disclosure(Disclosure),
    Divider,
}

impl From<Text> for Detail {
    fn from(text: Text) -> Self { Self::Text(text) }
}

/// A titled group the musician can open. It starts closed unless
/// `start_open`; `force_open` holds it open (previews capture open detail).
#[derive(Clone, Debug, PartialEq)]
pub struct Disclosure {
    pub title: String,
    /// Stable identity when the title carries a changing count.
    pub id_salt: Option<&'static str>,
    pub start_open: bool,
    pub force_open: bool,
    pub content: Vec<Detail>,
}

impl Disclosure {
    pub fn new(title: impl Into<String>, content: Vec<Detail>) -> Self {
        Self { title: title.into(), id_salt: None, start_open: false, force_open: false, content }
    }

    pub fn forced_open(mut self, force_open: bool) -> Self {
        self.force_open = force_open;
        self
    }
}

/// Exact identity of one installer setup attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptKey {
    pub installer: String,
    pub environment: Option<String>,
}

/// Where a navigation control leads: a page, or an exact item on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Destination {
    Product(ProductKey),
    /// Several exact plug-ins, such as the ones one installation found.
    Products(Vec<ProductKey>),
    Attempt(AttemptKey),
    Session(String),
    Activity,
    Setup,
    Diagnostics,
}

/// What a window-handled control does. None of these sends anything to the
/// manager; a manager action is always an `offers::Offer`.
#[derive(Clone, Debug, PartialEq)]
pub enum Local {
    Go(Destination),
    /// Read current status again, including the computer-setup check.
    CheckComputer,
    /// Choose a Windows installer file to add.
    ChooseInstaller,
    /// Choose a different installer, or a required app's installer, for this
    /// setup. The draft carries the setup's identity at the time of choosing.
    ChooseReplacement(setup::InstallerReplacement),
    /// Drop the replacement draft and keep the current installer.
    KeepCurrentInstaller,
    /// Stop showing only the selected installer.
    ShowAllSetup,
}

/// A control the window handles itself, such as opening another page.
#[derive(Clone, Debug, PartialEq)]
pub struct Control {
    pub label: &'static str,
    pub importance: offers::Importance,
    pub enabled: bool,
    pub does: Local,
}

impl Control {
    pub fn go(label: &'static str, importance: offers::Importance, destination: Destination) -> Self {
        Self { label, importance, enabled: true, does: Local::Go(destination) }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// One option of a choice the musician makes on this screen, with what the
/// screen shows once it is chosen.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice<T> {
    /// Stable identity of the option, such as a runtime or an installer.
    pub key: String,
    pub label: String,
    pub content: T,
}
