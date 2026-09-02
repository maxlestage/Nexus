//! Modèle de vue : description déclarative de l'écran, produite par Rust et
//! rendue telle quelle par SwiftUI.
//!
//! Aucun libellé ni aucune règle d'affichage ne vit côté Swift : l'interface
//! est entièrement décrite ici, ce qui la rend testable sans appareil.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct View {
    /// Écran courant : l'accueil de connexion, ou les onglets.
    pub screen: Screen,
    /// Bandeau affiché par-dessus tout (mise à jour en cours).
    pub banner: Option<Banner>,
    /// Message d'erreur à présenter, effacé par l'action `error.dismiss`.
    pub error: Option<String>,
    /// Une requête est en vol : l'interface montre une activité.
    pub busy: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Screen {
    /// Écran d'accueil tant que la manette n'est pas jointe.
    Connect {
        title: String,
        message: String,
        /// Bouton d'action, absent pendant une recherche en cours.
        action: Option<Row>,
        /// Une recherche ou une connexion est en cours.
        spinner: bool,
    },
    Tabs {
        tabs: Vec<Tab>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Tab {
    pub id: String,
    pub title: String,
    /// Nom de symbole SF, seul emprunt fait au vocabulaire d'Apple.
    pub icon: String,
    pub selected: bool,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct Section {
    pub header: Option<String>,
    pub footer: Option<String>,
    pub rows: Vec<Row>,
}

impl Section {
    pub fn new(header: impl Into<String>, rows: Vec<Row>) -> Self {
        Self {
            header: Some(header.into()),
            footer: None,
            rows,
        }
    }

    pub fn bare(rows: Vec<Row>) -> Self {
        Self {
            header: None,
            footer: None,
            rows,
        }
    }

    pub fn with_footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Option_ {
    pub value: String,
    pub label: String,
}

/// Une ligne de formulaire. `id` est renvoyé tel quel dans les actions.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Row {
    /// Texte simple, éventuellement avec une valeur alignée à droite.
    Text {
        label: String,
        value: Option<String>,
    },
    /// Choix parmi une liste.
    Picker {
        id: String,
        label: String,
        value: String,
        options: Vec<Option_>,
    },
    /// Choix binaire.
    Toggle {
        id: String,
        label: String,
        value: bool,
    },
    /// Valeur continue.
    Slider {
        id: String,
        label: String,
        value: f64,
        min: f64,
        max: f64,
        step: f64,
    },
    /// Valeur entière à incréments.
    Stepper {
        id: String,
        label: String,
        value: f64,
        min: f64,
        max: f64,
    },
    /// Bouton d'action. `destructive` colore en rouge, `confirm` demande une
    /// confirmation avant d'émettre l'action.
    Button {
        id: String,
        label: String,
        destructive: bool,
        disabled: bool,
        confirm: Option<Confirm>,
    },
    /// Saisie de texte.
    Field {
        id: String,
        label: String,
        value: String,
        placeholder: String,
        secure: bool,
        keyboard: String,
    },
    /// Couleur, au format `#rrggbb`.
    Color {
        id: String,
        label: String,
        value: String,
    },
    /// Barre de proportion, pour les statistiques.
    Gauge {
        label: String,
        value: f64,
        max: f64,
        detail: String,
    },
    /// Choix segmenté affiché en tête de page.
    Segmented {
        id: String,
        value: String,
        options: Vec<Option_>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Confirm {
    pub title: String,
    pub message: String,
    pub action_label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Banner {
    Ota {
        percent: u8,
        title: String,
        message: String,
    },
}
