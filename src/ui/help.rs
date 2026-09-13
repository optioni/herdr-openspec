//! The pane's key and gesture inventory, and the overlay's row grammar. See
//! `openspec/changes/help-overlay/specs/binding-inventory/spec.md` and
//! `specs/help-overlay/spec.md` -> "The overlay's row grammar is a rule row,
//! groups, and a rule row".
//!
//! Pure and total, on exactly `ui::palette`'s terms: no filesystem, process,
//! environment, network, or standard-I/O API, no clock, no global mutable
//! state, and no panic for any input.

#[cfg(test)]
mod tests {
    use super::{Binding, Group, INVENTORY, Scope};
    use crate::ui::app::Action;
    use std::collections::BTreeSet;

    /// The observable second site `specs/binding-inventory/spec.md`'s own
    /// scenario names: a const item is evaluated at compile time, so an
    /// `INVENTORY` that became a function call, allocated, or read a file
    /// would fail to compile here rather than pass a runtime assertion that
    /// has nothing to observe.
    const _: &[Group] = INVENTORY;

    #[test]
    fn the_inventory_s_shape_is_asserted_not_described() {
        let titles: Vec<&str> = INVENTORY.iter().map(|g| g.title).collect();
        assert_eq!(
            titles,
            vec!["Changes", "Artifact", "Agents", "Pane", "While filtering", "Mouse"]
        );

        let scopes: Vec<Scope> = INVENTORY.iter().map(|g| g.scope).collect();
        assert_eq!(
            scopes,
            vec![
                Scope::List,
                Scope::Detail,
                Scope::Any,
                Scope::Any,
                Scope::Filter,
                Scope::Any,
            ]
        );

        let counts: Vec<usize> = INVENTORY.iter().map(|g| g.bindings.len()).collect();
        assert_eq!(counts, vec![5, 7, 4, 4, 5, 6]);
        assert_eq!(counts.iter().sum::<usize>(), 31);

        assert!(
            INVENTORY.iter().all(|g| !g.bindings.is_empty()),
            "no group is empty"
        );
        let unique: BTreeSet<&str> = titles.iter().copied().collect();
        assert_eq!(unique.len(), titles.len(), "no two groups share a title");
    }

    #[test]
    fn space_and_esc_each_appear_under_their_route() {
        let space: Vec<(Scope, &Binding)> = INVENTORY
            .iter()
            .flat_map(|g| g.bindings.iter().map(move |b| (g.scope, b)))
            .filter(|(_, b)| b.input == "Space")
            .collect();
        assert_eq!(space.len(), 2);
        assert!(space.iter().any(|(scope, _)| *scope == Scope::List));
        assert!(space.iter().any(|(scope, _)| *scope == Scope::Detail));
        assert_ne!(
            space[0].1.description, space[1].1.description,
            "the two Space rows must describe different folds"
        );
        assert!(space.iter().all(|(_, b)| b.action == Action::ToggleSection));

        let esc: Vec<(Scope, &Binding)> = INVENTORY
            .iter()
            .flat_map(|g| g.bindings.iter().map(move |b| (g.scope, b)))
            .filter(|(_, b)| b.input.contains("Esc"))
            .collect();
        assert_eq!(esc.len(), 2);
        assert!(esc.iter().any(|(scope, _)| *scope == Scope::Detail));
        assert!(esc.iter().any(|(scope, _)| *scope == Scope::Filter));
        assert_ne!(
            esc[0].1.description, esc[1].1.description,
            "the two Esc rows must describe different effects"
        );
        assert!(esc.iter().all(|(_, b)| b.action == Action::Back));
    }

    #[test]
    fn both_quit_keys_have_a_row() {
        let quits: Vec<&Binding> = INVENTORY
            .iter()
            .flat_map(|g| g.bindings.iter())
            .filter(|b| b.action == Action::Quit)
            .collect();
        assert_eq!(quits.len(), 2);
        let inputs: BTreeSet<&str> = quits.iter().map(|b| b.input).collect();
        assert_eq!(inputs, BTreeSet::from(["q", "Ctrl-C"]));
        assert!(INVENTORY.iter().any(|g| g.title == "Pane"
            && g.scope == Scope::Any
            && g.bindings.iter().any(|b| b.action == Action::Quit)));
    }

    #[test]
    fn the_inventory_is_a_pure_static_value_with_no_construction_cost() {
        // `INVENTORY` is referenced from every test above with no
        // constructor call anywhere in this module — two independent
        // references to the `'static` value settle to the same address.
        let a: *const [Group] = INVENTORY;
        let b: *const [Group] = INVENTORY;
        assert_eq!(a, b);
    }
}
