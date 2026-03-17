use crate::connection::ConnectionConfig;
use dioxus::prelude::*;

#[component]
pub fn ConnectionDialog(
    on_close: EventHandler<()>,
    on_save: EventHandler<ConnectionConfig>,
    #[props(default)] existing: Option<ConnectionConfig>,
) -> Element {
    let is_edit = existing.is_some();
    let existing_id = existing.as_ref().map(|c| c.id);

    let mut name = use_signal(|| {
        existing
            .as_ref()
            .map(|c| c.name.clone())
            .unwrap_or_default()
    });
    let mut host = use_signal(|| {
        existing
            .as_ref()
            .map(|c| c.host.clone())
            .unwrap_or_else(|| "localhost".to_string())
    });
    let mut port = use_signal(|| {
        existing
            .as_ref()
            .map(|c| c.port.to_string())
            .unwrap_or_else(|| "9042".to_string())
    });
    let mut username = use_signal(|| {
        existing
            .as_ref()
            .and_then(|c| c.username.clone())
            .unwrap_or_default()
    });
    let mut password = use_signal(|| {
        existing
            .as_ref()
            .and_then(|c| c.password.clone())
            .unwrap_or_default()
    });
    let mut keyspace = use_signal(|| {
        existing
            .as_ref()
            .and_then(|c| c.keyspace.clone())
            .unwrap_or_default()
    });
    let mut validation_error = use_signal(|| None::<String>);

    let title = if is_edit { "Edit Connection" } else { "New Connection" };

    rsx! {
        div {
            class: "modal-backdrop",
            onclick: move |_| on_close.call(()),

            div {
                class: "modal",
                onclick: move |e| e.stop_propagation(),

                div {
                    class: "modal-header",
                    h2 { "{title}" }
                    button {
                        class: "btn-close",
                        onclick: move |_| on_close.call(()),
                        "x"
                    }
                }

                div {
                    class: "modal-body",

                    div {
                        class: "form-group",
                        label { "Name" }
                        input {
                            r#type: "text",
                            value: "{name.read()}",
                            oninput: move |e| name.set(e.value()),
                            placeholder: "My Cassandra Cluster"
                        }
                    }

                    div {
                        class: "form-row",
                        div {
                            class: "form-group",
                            label { "Host" }
                            input {
                                r#type: "text",
                                value: "{host.read()}",
                                oninput: move |e| host.set(e.value()),
                                placeholder: "localhost"
                            }
                        }

                        div {
                            class: "form-group form-group-small",
                            label { "Port" }
                            input {
                                r#type: "text",
                                value: "{port.read()}",
                                oninput: move |e| port.set(e.value()),
                                placeholder: "9042"
                            }
                        }
                    }

                    div {
                        class: "form-group",
                        label { "Username (optional)" }
                        input {
                            r#type: "text",
                            value: "{username.read()}",
                            oninput: move |e| username.set(e.value())
                        }
                    }

                    div {
                        class: "form-group",
                        label { "Password (optional)" }
                        input {
                            r#type: "password",
                            value: "{password.read()}",
                            oninput: move |e| password.set(e.value())
                        }
                    }

                    div {
                        class: "form-group",
                        label { "Default Keyspace (optional)" }
                        input {
                            r#type: "text",
                            value: "{keyspace.read()}",
                            oninput: move |e| keyspace.set(e.value())
                        }
                    }
                }

                // Validation error message
                if let Some(error) = validation_error.read().as_ref() {
                    div {
                        class: "form-error",
                        "{error}"
                    }
                }

                div {
                    class: "modal-footer",

                    button {
                        class: "btn btn-secondary",
                        onclick: move |_| on_close.call(()),
                        "Cancel"
                    }

                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            // Validate required fields
                            if name.read().trim().is_empty() {
                                validation_error.set(Some("Connection name is required".to_string()));
                                return;
                            }
                            if host.read().trim().is_empty() {
                                validation_error.set(Some("Host is required".to_string()));
                                return;
                            }
                            let port_num = match port.read().parse::<u16>() {
                                Ok(p) if p > 0 => p,
                                _ => {
                                    validation_error.set(Some("Port must be a valid number (1-65535)".to_string()));
                                    return;
                                }
                            };
                            // Validate keyspace if provided
                            let ks = keyspace.read().trim().to_string();
                            if !ks.is_empty() {
                                if let Err(e) = crate::cassandra::validate_cql_identifier(&ks) {
                                    validation_error.set(Some(format!("Invalid keyspace: {}", e)));
                                    return;
                                }
                            }

                            validation_error.set(None);

                            let mut config = if let Some(id) = existing_id {
                                // Preserve the original ID when editing
                                let mut c = ConnectionConfig::new(
                                    name.read().trim().to_string(),
                                    host.read().trim().to_string(),
                                );
                                c.id = id;
                                c
                            } else {
                                ConnectionConfig::new(
                                    name.read().trim().to_string(),
                                    host.read().trim().to_string(),
                                )
                            };
                            config.port = port_num;

                            if !username.read().is_empty() {
                                config.username = Some(username.read().clone());
                                config.password = Some(password.read().clone());
                            }

                            if !ks.is_empty() {
                                config.keyspace = Some(ks);
                            }

                            on_save.call(config);
                        },
                        if is_edit { "Update" } else { "Save" }
                    }
                }
            }
        }
    }
}
