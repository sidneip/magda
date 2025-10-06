use dioxus::prelude::*;
use crate::connection::ConnectionConfig;

#[component]
pub fn ConnectionDialog(
    on_close: EventHandler<()>,
    on_save: EventHandler<ConnectionConfig>
) -> Element {
    let mut name = use_signal(String::new);
    let mut host = use_signal(|| "localhost".to_string());
    let mut port = use_signal(|| "9042".to_string());
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut keyspace = use_signal(String::new);
    
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
                            let mut config = ConnectionConfig::new(
                                name.read().clone(),
                                host.read().clone()
                            );
                            
                            if let Ok(port_num) = port.read().parse::<u16>() {
                                config.port = port_num;
                            }
                            
                            if !username.read().is_empty() {
                                config.username = Some(username.read().clone());
                                config.password = Some(password.read().clone());
                            }
                            
                            if !keyspace.read().is_empty() {
                                config.keyspace = Some(keyspace.read().clone());
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