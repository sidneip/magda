use crate::connection::ConnectionConfig;
use dioxus::prelude::*;

#[component]
pub fn ConnectionDialog(
    on_close: EventHandler<()>,
    on_save: EventHandler<ConnectionConfig>,
) -> Element {
    let mut name = use_signal(String::new);
    let mut host = use_signal(|| "localhost".to_string());
    let mut port = use_signal(|| "9042".to_string());
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut keyspace = use_signal(String::new);
    let mut validation_error = use_signal(|| None::<String>);

    rsx! {
        div {
            class: "modal-backdrop",
            onclick: move |_| on_close.call(()),

            div {
                class: "modal",
                onclick: move |e| e.stop_propagation(),

                div {
                    class: "modal-header",
                    h2 { "New Connection" }
                    button {
                        class: "btn-close",
                        onclick: move |_| on_close.call(()),
                        "×"
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

                            let mut config = ConnectionConfig::new(
                                name.read().trim().to_string(),
                                host.read().trim().to_string(),
                            );
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
                        "Save"
                    }
                }
            }
        }
    }
}
