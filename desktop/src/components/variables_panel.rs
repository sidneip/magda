use dioxus::prelude::*;

use crate::config::save_variables;
use crate::state::{AppState, QueryVariable};

#[component]
pub fn VariablesPanel() -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let mut variables = app_state.read().query_variables.clone();

    let add_variable = move |_| {
        variables.write().push(QueryVariable {
            name: String::new(),
            value: String::new(),
        });
        save_variables(&variables.read());
    };

    let mut delete_variable = move |idx: usize| {
        variables.write().remove(idx);
        save_variables(&variables.read());
    };

    rsx! {
        div {
            class: "variables-panel",

            div {
                class: "variables-header",
                h3 { "Variables" }
                button {
                    class: "btn btn-small",
                    onclick: add_variable,
                    "+ Add"
                }
            }

            if variables.read().is_empty() {
                div {
                    class: "empty-state",
                    "No variables defined yet. Add a variable and use {{name}} in your queries."
                }
            } else {
                div {
                    class: "variables-list",

                    for (i, var) in variables.read().iter().enumerate() {
                        {
                            let name = var.name.clone();
                            let value = var.value.clone();
                            rsx! {
                                div {
                                    key: "{i}",
                                    class: "variable-row",

                                    input {
                                        class: "variable-input variable-name-input",
                                        r#type: "text",
                                        placeholder: "name",
                                        value: "{name}",
                                        onchange: move |e: Event<FormData>| {
                                            variables.write()[i].name = e.value().to_string();
                                            save_variables(&variables.read());
                                        }
                                    }

                                    input {
                                        class: "variable-input variable-value-input",
                                        r#type: "text",
                                        placeholder: "value",
                                        value: "{value}",
                                        onchange: move |e: Event<FormData>| {
                                            variables.write()[i].value = e.value().to_string();
                                            save_variables(&variables.read());
                                        }
                                    }

                                    button {
                                        class: "btn-small btn-danger",
                                        onclick: move |_| delete_variable(i),
                                        "Delete"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
