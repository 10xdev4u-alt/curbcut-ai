//! Role refinement from visual cues.
//!
//! The platform role is the starting point. Visual cues refine it: a positive
//! heading level makes an element a heading, a button that owns children
//! becomes a menu item, and a text field the platform says is not editable
//! becomes static text.
//!
//! [`Role::Unknown`] is never refined. An unmapped platform role is a signal,
//! not something to guess away, so it is returned unchanged for the provenance
//! panel.

use curbcut_core::types::Role;

/// Visual cues a platform adapter can report without content.
///
/// `editable` is `Option<bool>` because "the adapter did not report it" and
/// "the adapter reported not editable" are different facts. Treating an
/// unreported value as `false` would silently relabel every text field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cues {
    /// The element has children.
    pub has_children: bool,
    /// The platform reports a heading level, if any.
    pub heading_level: Option<u8>,
    /// The platform reports whether the element is editable, if it reported it.
    pub editable: Option<bool>,
}

impl Cues {
    /// No cues.
    pub fn none() -> Self {
        Self::default()
    }

    /// A content-free summary for rule prose: flags and counts only.
    pub fn summary(&self) -> String {
        let heading = self
            .heading_level
            .map_or_else(|| "none".to_string(), |level| level.to_string());
        let editable = self
            .editable
            .map_or_else(|| "unknown".to_string(), |value| value.to_string());
        format!(
            "children={},heading={},editable={}",
            self.has_children, heading, editable
        )
    }
}

/// Refines a role from cues. Pure.
///
/// The function never invents a role the platform did not support: it only
/// promotes within the core vocabulary, and it leaves [`Role::Unknown`] alone.
pub fn refine_role(role: Role, cues: &Cues) -> Role {
    if matches!(role, Role::Unknown(_)) {
        return role;
    }
    if let Some(level) = cues.heading_level {
        if level > 0 {
            return Role::Heading;
        }
    }
    match role {
        Role::TextField if cues.editable == Some(false) => Role::StaticText,
        Role::Button if cues.has_children => Role::MenuItem,
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_heading_level_promotes_any_known_role() {
        let cues = Cues {
            heading_level: Some(2),
            ..Cues::none()
        };
        assert_eq!(refine_role(Role::StaticText, &cues), Role::Heading);
        assert_eq!(refine_role(Role::TextField, &cues), Role::Heading);
        assert_eq!(refine_role(Role::Button, &cues), Role::Heading);
    }

    #[test]
    fn a_non_editable_text_field_is_static_text() {
        let cues = Cues {
            editable: Some(false),
            ..Cues::none()
        };
        assert_eq!(refine_role(Role::TextField, &cues), Role::StaticText);
    }

    #[test]
    fn an_editable_text_field_stays_a_text_field() {
        let cues = Cues {
            editable: Some(true),
            ..Cues::none()
        };
        assert_eq!(refine_role(Role::TextField, &cues), Role::TextField);
    }

    #[test]
    fn an_unreported_editable_value_changes_nothing() {
        let cues = Cues {
            editable: None,
            ..Cues::none()
        };
        assert_eq!(refine_role(Role::TextField, &cues), Role::TextField);
    }

    #[test]
    fn a_button_with_children_is_a_menu_item() {
        let cues = Cues {
            has_children: true,
            ..Cues::none()
        };
        assert_eq!(refine_role(Role::Button, &cues), Role::MenuItem);
        assert_eq!(refine_role(Role::Link, &cues), Role::Link);
    }

    #[test]
    fn an_unknown_role_is_never_guessed() {
        let cues = Cues {
            heading_level: Some(1),
            has_children: true,
            editable: Some(true),
        };
        let unknown = Role::Unknown("AXCustom".to_string());
        assert_eq!(refine_role(unknown.clone(), &cues), unknown);
    }

    #[test]
    fn no_cues_changes_nothing() {
        for role in [
            Role::Button,
            Role::Link,
            Role::TextField,
            Role::Checkbox,
            Role::Radio,
            Role::Switch,
            Role::MenuItem,
            Role::Tab,
            Role::Dialog,
            Role::Alert,
            Role::Image,
            Role::Icon,
            Role::StaticText,
            Role::Heading,
            Role::List,
            Role::ListItem,
            Role::Table,
        ] {
            assert_eq!(refine_role(role.clone(), &Cues::none()), role);
        }
    }

    #[test]
    fn a_heading_level_of_zero_is_not_a_heading() {
        let cues = Cues {
            heading_level: Some(0),
            ..Cues::none()
        };
        assert_eq!(refine_role(Role::StaticText, &cues), Role::StaticText);
    }

    #[test]
    fn the_summary_is_content_free() {
        let cues = Cues {
            has_children: true,
            heading_level: Some(3),
            editable: Some(false),
        };
        let summary = cues.summary();
        assert!(summary.contains("children=true"), "{summary}");
        assert!(summary.contains("heading=3"), "{summary}");
        assert!(summary.contains("editable=false"), "{summary}");
        assert!(Cues::none().summary().contains("editable=unknown"));
    }
}
