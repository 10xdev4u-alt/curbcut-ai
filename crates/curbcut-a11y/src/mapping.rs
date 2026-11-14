//! Platform role and state mapping.
//!
//! Each platform reports accessibility roles and states in its own vocabulary.
//! This module maps both onto the core [`Role`] and [`ElementState`] so the
//! merge, understand, and adapt pipelines are platform-independent.
//!
//! The raw strings are the platform's own names:
//!
//! - macOS: the `AXRole` attribute value, for example `AXButton`.
//! - Windows: the UIA `ControlType` name, for example `Button`.
//! - Linux: the AT-SPI role name, for example `push-button`.
//!
//! A raw role this table does not know becomes [`Role::Unknown`] with the raw
//! string preserved. An unmapped role is therefore flagged, never dropped.
//!
//! The compatibility tables are `ROLE_COMPAT` and `STATE_COMPAT`. They are
//! data, not code, so a new platform role is a table row and a test, not a new
//! branch. `docs/ROLE_MAPPING.md` is rendered from the same rows by
//! `scripts/render_role_mapping.py`.

use std::fmt;

use curbcut_core::types::{ElementState, Role};

/// A platform that produces accessibility trees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Platform {
    /// macOS AXUIElement.
    MacOs,
    /// Windows UI Automation.
    Windows,
    /// Linux AT-SPI.
    Linux,
}

impl Platform {
    /// The stable string form.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MacOs => "macos",
            Self::Windows => "windows",
            Self::Linux => "linux",
        }
    }

    /// Whether an element is enabled when the platform reports no enabled state.
    ///
    /// macOS and Windows expose enabled as a positive attribute that is absent
    /// when the element is disabled, so an element is enabled unless it says
    /// otherwise. AT-SPI exposes `enabled` as a member of the state set and
    /// omits it when the element is disabled, so a Linux element with no
    /// enabled state is disabled.
    pub fn default_enabled(self) -> bool {
        match self {
            Self::MacOs | Self::Windows => true,
            Self::Linux => false,
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One row of the role compatibility table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleMapping {
    /// The platform whose raw role this is.
    pub platform: Platform,
    /// The platform's raw role string.
    pub raw: &'static str,
    /// The core role it maps to. Never [`Role::Unknown`].
    pub core: Role,
}

/// The role compatibility table.
///
/// Every row maps to a concrete core role. A platform role that is absent from
/// this table is reported as [`Role::Unknown`] with its raw string, so it is
/// flagged rather than silently dropped.
pub static ROLE_COMPAT: &[RoleMapping] = &[
    // macOS AXRole values.
    RoleMapping {
        platform: Platform::MacOs,
        raw: "AXButton",
        core: Role::Button,
    },
    RoleMapping {
        platform: Platform::MacOs,
        raw: "AXLink",
        core: Role::Link,
    },
    RoleMapping {
        platform: Platform::MacOs,
        raw: "AXTextField",
        core: Role::TextField,
    },
    RoleMapping {
        platform: Platform::MacOs,
        raw: "AXTextArea",
        core: Role::TextField,
    },
    RoleMapping {
        platform: Platform::MacOs,
        raw: "AXCheckBox",
        core: Role::Checkbox,
    },
    RoleMapping {
        platform: Platform::MacOs,
        raw: "AXRadioButton",
        core: Role::Radio,
    },
    RoleMapping {
        platform: Platform::MacOs,
        raw: "AXSwitch",
        core: Role::Switch,
    },
    RoleMapping {
        platform: Platform::MacOs,
        raw: "AXMenuButton",
        core: Role::Button,
    },
    RoleMapping {
        platform: Platform::MacOs,
        raw: "AXMenuItem",
        core: Role::MenuItem,
    },
    RoleMapping {
        platform: Platform::MacOs,
        raw: "AXMenuBarItem",
        core: Role::MenuItem,
    },
    RoleMapping {
        platform: Platform::MacOs,
        raw: "AXTabGroup",
        core: Role::Tab,
    },
    RoleMapping {
        platform: Platform::MacOs,
        raw: "AXDialog",
        core: Role::Dialog,
    },
    RoleMapping {
        platform: Platform::MacOs,
        raw: "AXSheet",
        core: Role::Dialog,
    },
    RoleMapping {
        platform: Platform::MacOs,
        raw: "AXAlert",
        core: Role::Alert,
    },
    RoleMapping {
        platform: Platform::MacOs,
        raw: "AXImage",
        core: Role::Image,
    },
    RoleMapping {
        platform: Platform::MacOs,
        raw: "AXIcon",
        core: Role::Icon,
    },
    RoleMapping {
        platform: Platform::MacOs,
        raw: "AXStaticText",
        core: Role::StaticText,
    },
    RoleMapping {
        platform: Platform::MacOs,
        raw: "AXHeading",
        core: Role::Heading,
    },
    RoleMapping {
        platform: Platform::MacOs,
        raw: "AXList",
        core: Role::List,
    },
    RoleMapping {
        platform: Platform::MacOs,
        raw: "AXRow",
        core: Role::ListItem,
    },
    RoleMapping {
        platform: Platform::MacOs,
        raw: "AXTable",
        core: Role::Table,
    },
    // Windows UIA ControlType names.
    RoleMapping {
        platform: Platform::Windows,
        raw: "Button",
        core: Role::Button,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "SplitButton",
        core: Role::Button,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "Hyperlink",
        core: Role::Link,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "Edit",
        core: Role::TextField,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "ComboBox",
        core: Role::TextField,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "CheckBox",
        core: Role::Checkbox,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "RadioButton",
        core: Role::Radio,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "ListItem",
        core: Role::ListItem,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "DataItem",
        core: Role::ListItem,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "TreeItem",
        core: Role::ListItem,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "MenuItem",
        core: Role::MenuItem,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "Tab",
        core: Role::Tab,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "TabItem",
        core: Role::Tab,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "Window",
        core: Role::Dialog,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "Text",
        core: Role::StaticText,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "Document",
        core: Role::StaticText,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "TitleBar",
        core: Role::StaticText,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "Image",
        core: Role::Image,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "List",
        core: Role::List,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "Menu",
        core: Role::List,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "MenuBar",
        core: Role::List,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "Tree",
        core: Role::List,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "DataGrid",
        core: Role::Table,
    },
    RoleMapping {
        platform: Platform::Windows,
        raw: "Table",
        core: Role::Table,
    },
    // Linux AT-SPI role names.
    RoleMapping {
        platform: Platform::Linux,
        raw: "push-button",
        core: Role::Button,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "toggle-button",
        core: Role::Switch,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "check-box",
        core: Role::Checkbox,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "radio-button",
        core: Role::Radio,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "combo-box",
        core: Role::TextField,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "password-text",
        core: Role::TextField,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "text-box",
        core: Role::TextField,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "link",
        core: Role::Link,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "list",
        core: Role::List,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "list-item",
        core: Role::ListItem,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "menu",
        core: Role::List,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "menu-bar",
        core: Role::List,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "menu-item",
        core: Role::MenuItem,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "page-tab",
        core: Role::Tab,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "page-tab-list",
        core: Role::Tab,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "table",
        core: Role::Table,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "table-cell",
        core: Role::ListItem,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "tree",
        core: Role::List,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "tree-table",
        core: Role::Table,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "dialog",
        core: Role::Dialog,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "alert",
        core: Role::Alert,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "image",
        core: Role::Image,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "icon",
        core: Role::Icon,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "label",
        core: Role::StaticText,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "heading",
        core: Role::Heading,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "paragraph",
        core: Role::StaticText,
    },
    RoleMapping {
        platform: Platform::Linux,
        raw: "section",
        core: Role::StaticText,
    },
];

/// Maps a platform role onto the core role.
///
/// Unknown roles come back as [`Role::Unknown`] carrying the raw string.
pub fn map_role(platform: Platform, raw: &str) -> Role {
    ROLE_COMPAT
        .iter()
        .find(|row| row.platform == platform && row.raw == raw)
        .map(|row| row.core.clone())
        .unwrap_or_else(|| Role::Unknown(raw.to_string()))
}

/// Whether a platform role has an explicit mapping.
pub fn is_mapped(platform: Platform, raw: &str) -> bool {
    ROLE_COMPAT
        .iter()
        .any(|row| row.platform == platform && row.raw == raw)
}

/// Every mapping row for a platform.
pub fn role_mappings(platform: Platform) -> impl Iterator<Item = &'static RoleMapping> {
    ROLE_COMPAT
        .iter()
        .filter(move |row| row.platform == platform)
}

/// The state flags an element can report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateFlag {
    /// Enabled.
    Enabled,
    /// Focused.
    Focused,
    /// Selected.
    Selected,
    /// Expanded.
    Expanded,
}

/// One row of the state compatibility table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateMapping {
    /// The platform whose raw state this is.
    pub platform: Platform,
    /// The platform's raw state name.
    pub raw: &'static str,
    /// The flag to set.
    pub flag: StateFlag,
    /// The value to set it to.
    pub value: bool,
}

/// The state compatibility table.
pub static STATE_COMPAT: &[StateMapping] = &[
    // macOS AX state attributes.
    StateMapping {
        platform: Platform::MacOs,
        raw: "AXEnabled",
        flag: StateFlag::Enabled,
        value: true,
    },
    StateMapping {
        platform: Platform::MacOs,
        raw: "AXDisabled",
        flag: StateFlag::Enabled,
        value: false,
    },
    StateMapping {
        platform: Platform::MacOs,
        raw: "AXFocused",
        flag: StateFlag::Focused,
        value: true,
    },
    StateMapping {
        platform: Platform::MacOs,
        raw: "AXSelected",
        flag: StateFlag::Selected,
        value: true,
    },
    StateMapping {
        platform: Platform::MacOs,
        raw: "AXExpanded",
        flag: StateFlag::Expanded,
        value: true,
    },
    StateMapping {
        platform: Platform::MacOs,
        raw: "AXCollapsed",
        flag: StateFlag::Expanded,
        value: false,
    },
    // Windows UIA state property names.
    StateMapping {
        platform: Platform::Windows,
        raw: "IsEnabled",
        flag: StateFlag::Enabled,
        value: true,
    },
    StateMapping {
        platform: Platform::Windows,
        raw: "IsDisabled",
        flag: StateFlag::Enabled,
        value: false,
    },
    StateMapping {
        platform: Platform::Windows,
        raw: "HasKeyboardFocus",
        flag: StateFlag::Focused,
        value: true,
    },
    StateMapping {
        platform: Platform::Windows,
        raw: "IsSelected",
        flag: StateFlag::Selected,
        value: true,
    },
    StateMapping {
        platform: Platform::Windows,
        raw: "IsExpanded",
        flag: StateFlag::Expanded,
        value: true,
    },
    StateMapping {
        platform: Platform::Windows,
        raw: "IsCollapsed",
        flag: StateFlag::Expanded,
        value: false,
    },
    // Linux AT-SPI state names.
    StateMapping {
        platform: Platform::Linux,
        raw: "enabled",
        flag: StateFlag::Enabled,
        value: true,
    },
    StateMapping {
        platform: Platform::Linux,
        raw: "disabled",
        flag: StateFlag::Enabled,
        value: false,
    },
    StateMapping {
        platform: Platform::Linux,
        raw: "focused",
        flag: StateFlag::Focused,
        value: true,
    },
    StateMapping {
        platform: Platform::Linux,
        raw: "selected",
        flag: StateFlag::Selected,
        value: true,
    },
    StateMapping {
        platform: Platform::Linux,
        raw: "expanded",
        flag: StateFlag::Expanded,
        value: true,
    },
    StateMapping {
        platform: Platform::Linux,
        raw: "collapsed",
        flag: StateFlag::Expanded,
        value: false,
    },
];

/// Maps a set of platform state names onto [`ElementState`].
///
/// `states` holds the state names the platform reports as present. An
/// unrecognized name is ignored, so a new platform state cannot corrupt the
/// element.
///
/// The starting value is [`Platform::default_enabled`]: macOS and Windows are
/// enabled unless they say otherwise, while AT-SPI omits `enabled` for a
/// disabled element, so Linux starts disabled. The adapter reports negative
/// facts through the table's negative rows; for example, an AT-SPI element
/// with no `enabled` state stays disabled without the adapter injecting
/// `disabled`. When a platform reports contradictory names, the last name in
/// `states` wins.
pub fn map_state(platform: Platform, states: &[&str]) -> ElementState {
    let mut state = ElementState {
        enabled: platform.default_enabled(),
        ..ElementState::default()
    };
    for raw in states {
        if let Some(row) = STATE_COMPAT
            .iter()
            .find(|row| row.platform == platform && row.raw == *raw)
        {
            match row.flag {
                StateFlag::Enabled => state.enabled = row.value,
                StateFlag::Focused => state.focused = row.value,
                StateFlag::Selected => state.selected = row.value,
                StateFlag::Expanded => state.expanded = row.value,
            }
        }
    }
    state
}

/// Every state mapping row for a platform.
pub fn state_mappings(platform: Platform) -> impl Iterator<Item = &'static StateMapping> {
    STATE_COMPAT
        .iter()
        .filter(move |row| row.platform == platform)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_table_row_maps_to_a_concrete_role() {
        for row in ROLE_COMPAT {
            assert!(
                !matches!(row.core, Role::Unknown(_)),
                "{} role `{}` must map to a concrete core role",
                row.platform,
                row.raw
            );
        }
    }

    #[test]
    fn every_platform_has_a_non_empty_role_table() {
        for platform in [Platform::MacOs, Platform::Windows, Platform::Linux] {
            assert!(
                role_mappings(platform).count() >= 10,
                "{platform} must have a real role table"
            );
        }
    }

    #[test]
    fn macos_roles_map() {
        assert_eq!(map_role(Platform::MacOs, "AXButton"), Role::Button);
        assert_eq!(map_role(Platform::MacOs, "AXTextField"), Role::TextField);
        assert_eq!(map_role(Platform::MacOs, "AXCheckBox"), Role::Checkbox);
        assert_eq!(map_role(Platform::MacOs, "AXRadioButton"), Role::Radio);
        assert_eq!(map_role(Platform::MacOs, "AXSwitch"), Role::Switch);
        assert_eq!(map_role(Platform::MacOs, "AXMenuItem"), Role::MenuItem);
        assert_eq!(map_role(Platform::MacOs, "AXStaticText"), Role::StaticText);
        assert_eq!(map_role(Platform::MacOs, "AXHeading"), Role::Heading);
        assert_eq!(map_role(Platform::MacOs, "AXList"), Role::List);
        assert_eq!(map_role(Platform::MacOs, "AXTable"), Role::Table);
        assert_eq!(map_role(Platform::MacOs, "AXImage"), Role::Image);
    }

    #[test]
    fn windows_roles_map() {
        assert_eq!(map_role(Platform::Windows, "Button"), Role::Button);
        assert_eq!(map_role(Platform::Windows, "Edit"), Role::TextField);
        assert_eq!(map_role(Platform::Windows, "CheckBox"), Role::Checkbox);
        assert_eq!(map_role(Platform::Windows, "RadioButton"), Role::Radio);
        assert_eq!(map_role(Platform::Windows, "ListItem"), Role::ListItem);
        assert_eq!(map_role(Platform::Windows, "MenuItem"), Role::MenuItem);
        assert_eq!(map_role(Platform::Windows, "TabItem"), Role::Tab);
        assert_eq!(map_role(Platform::Windows, "Text"), Role::StaticText);
        assert_eq!(map_role(Platform::Windows, "Hyperlink"), Role::Link);
        assert_eq!(map_role(Platform::Windows, "DataGrid"), Role::Table);
    }

    #[test]
    fn linux_roles_map() {
        assert_eq!(map_role(Platform::Linux, "push-button"), Role::Button);
        assert_eq!(map_role(Platform::Linux, "toggle-button"), Role::Switch);
        assert_eq!(map_role(Platform::Linux, "check-box"), Role::Checkbox);
        assert_eq!(map_role(Platform::Linux, "radio-button"), Role::Radio);
        assert_eq!(map_role(Platform::Linux, "text-box"), Role::TextField);
        assert_eq!(map_role(Platform::Linux, "link"), Role::Link);
        assert_eq!(map_role(Platform::Linux, "list-item"), Role::ListItem);
        assert_eq!(map_role(Platform::Linux, "menu-item"), Role::MenuItem);
        assert_eq!(map_role(Platform::Linux, "page-tab"), Role::Tab);
        assert_eq!(map_role(Platform::Linux, "table"), Role::Table);
        assert_eq!(map_role(Platform::Linux, "heading"), Role::Heading);
    }

    #[test]
    fn unmapped_roles_are_flagged_with_the_raw_string() {
        let role = map_role(Platform::MacOs, "AXCustomWidget");
        assert_eq!(role, Role::Unknown("AXCustomWidget".to_string()));
        assert_eq!(role.as_str(), "AXCustomWidget");
        assert!(!is_mapped(Platform::MacOs, "AXCustomWidget"));

        let role = map_role(Platform::Windows, "ProgressBar");
        assert_eq!(role, Role::Unknown("ProgressBar".to_string()));

        let role = map_role(Platform::Linux, "scroll-bar");
        assert_eq!(role, Role::Unknown("scroll-bar".to_string()));
    }

    #[test]
    fn the_same_raw_string_can_map_differently_per_platform() {
        // "Text" is a Windows static-text role; macOS uses AXStaticText and
        // Linux uses "paragraph". The platform is part of the key.
        assert_eq!(map_role(Platform::Windows, "Text"), Role::StaticText);
        assert_eq!(
            map_role(Platform::MacOs, "Text"),
            Role::Unknown("Text".to_string())
        );
        assert_eq!(
            map_role(Platform::Linux, "Text"),
            Role::Unknown("Text".to_string())
        );
    }

    #[test]
    fn macos_states_map() {
        let state = map_state(Platform::MacOs, &["AXEnabled", "AXFocused"]);
        assert!(state.enabled);
        assert!(state.focused);
        assert!(!state.selected);

        let state = map_state(Platform::MacOs, &["AXDisabled"]);
        assert!(!state.enabled);

        let state = map_state(Platform::MacOs, &["AXSelected", "AXExpanded"]);
        assert!(state.selected);
        assert!(state.expanded);

        let state = map_state(Platform::MacOs, &["AXCollapsed"]);
        assert!(!state.expanded);
    }

    #[test]
    fn windows_states_map() {
        let state = map_state(Platform::Windows, &["IsEnabled", "HasKeyboardFocus"]);
        assert!(state.enabled);
        assert!(state.focused);

        let state = map_state(Platform::Windows, &["IsDisabled", "IsSelected"]);
        assert!(!state.enabled);
        assert!(state.selected);

        let state = map_state(Platform::Windows, &["IsCollapsed"]);
        assert!(!state.expanded);
    }

    #[test]
    fn linux_states_map() {
        let state = map_state(Platform::Linux, &["enabled", "focused", "selected"]);
        assert!(state.enabled);
        assert!(state.focused);
        assert!(state.selected);

        let state = map_state(Platform::Linux, &["disabled", "collapsed"]);
        assert!(!state.enabled);
        assert!(!state.expanded);
    }

    #[test]
    fn an_unknown_state_is_ignored() {
        let state = map_state(Platform::Linux, &["enabled", "brand-new-state"]);
        assert!(state.enabled);
        let state = map_state(Platform::Windows, &["IsEnabled", "future-state"]);
        assert!(state.enabled);
    }

    #[test]
    fn an_empty_state_set_follows_the_platform_default() {
        assert!(Platform::MacOs.default_enabled());
        assert!(Platform::Windows.default_enabled());
        assert!(!Platform::Linux.default_enabled());

        for platform in [Platform::MacOs, Platform::Windows, Platform::Linux] {
            let state = map_state(platform, &[]);
            assert_eq!(
                state.enabled,
                platform.default_enabled(),
                "{platform} must start from its own default"
            );
            assert!(!state.focused);
            assert!(!state.selected);
            assert!(!state.expanded);
        }
    }

    #[test]
    fn a_linux_element_with_no_enabled_state_is_disabled() {
        // AT-SPI omits `enabled` for a disabled element, so the adapter does
        // not need to inject `disabled`.
        assert!(!map_state(Platform::Linux, &[]).enabled);
        assert!(map_state(Platform::Linux, &["enabled"]).enabled);
    }

    #[test]
    fn every_state_row_has_a_known_flag() {
        assert!(!STATE_COMPAT.is_empty());
        for platform in [Platform::MacOs, Platform::Windows, Platform::Linux] {
            assert!(
                state_mappings(platform).count() >= 5,
                "{platform} must have a real state table"
            );
        }
    }

    #[test]
    fn platform_names_are_stable() {
        assert_eq!(Platform::MacOs.as_str(), "macos");
        assert_eq!(Platform::Windows.as_str(), "windows");
        assert_eq!(Platform::Linux.as_str(), "linux");
        assert_eq!(Platform::MacOs.to_string(), "macos");
    }
}
