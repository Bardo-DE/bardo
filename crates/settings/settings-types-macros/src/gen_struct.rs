//! Codegen for `#[derive(SettingsSchema)]` on a struct: one `impl SettingsSchema` whose
//! `schema()` builds a `KeySpec` per field from its `#[settings(...)]` attributes and its own
//! type (design/22-SETTINGS.md section 9.1).

use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

use crate::attrs::{EndsAttr, FieldAttrs, FootnoteAttr, container_attrs, field_attrs};
use crate::shape::{Shape, shape_of};

#[expect(
    clippy::too_many_lines,
    reason = "one arm for each attribute key (or field kind), each reading its own value"
)]
pub(crate) fn expand(input: &syn::DeriveInput, data: &syn::DataStruct) -> syn::Result<TokenStream> {
    let syn::Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new(
            data.fields.span(),
            "#[derive(SettingsSchema)] only supports a struct with named fields",
        ));
    };

    let container = container_attrs(input.span(), &input.attrs)?;
    let app = container
        .file
        .split('/')
        .next()
        .unwrap_or(&container.file)
        .to_owned();
    let file = &container.file;
    let domain = &container.domain;
    let page = &container.page;
    let ident = &input.ident;

    let mut pushes = Vec::new();
    let mut guards = Vec::new();
    for field in &fields.named {
        let Some(field_ident) = &field.ident else {
            continue;
        };
        let attrs = field_attrs(field.span(), &field.attrs)?;
        let FieldAttrs::Key {
            label,
            help,
            section,
            unit,
            advanced,
            agent_settable,
            hint,
            control,
            note,
            ends,
        } = attrs
        else {
            // `#[settings(skip)]`: a catch-all field such as `extra: toml::Table`, not a key.
            continue;
        };
        let path = format!("{domain}.{field_ident}");
        let exposure = if advanced {
            quote! { ::settings_types::schema::Exposure::Advanced }
        } else {
            quote! { ::settings_types::schema::Exposure::Basic }
        };
        let agent_setting = if agent_settable {
            guards.push(never_settable_guard(&path));
            quote! { ::settings_types::schema::AgentSetting::Settable }
        } else {
            quote! { ::settings_types::schema::AgentSetting::HandsOff }
        };
        let shape = shape_of(&field_ident.to_string(), &field.ty, hint)
            .map_err(|reason| syn::Error::new(field.ty.span(), reason))?;
        let kind = kind_expr(shape, &field.ty, unit.as_deref());
        let control = if let Some(control) = control {
            quote! {
                ::std::option::Option::Some(::settings_types::schema::ControlName(#control.to_owned()))
            }
        } else {
            quote! { ::std::option::Option::None }
        };
        let note = if let Some(note) = note {
            quote! {
                ::std::option::Option::Some(::settings_types::schema::Note(#note.to_owned()))
            }
        } else {
            quote! { ::std::option::Option::None }
        };
        let ends = if let Some(EndsAttr { low, high }) = ends.map(|ends| *ends) {
            quote! {
                ::std::option::Option::Some(::settings_types::schema::SliderEnds {
                    low: ::settings_types::schema::Label(#low.to_owned()),
                    high: ::settings_types::schema::Label(#high.to_owned()),
                })
            }
        } else {
            quote! { ::std::option::Option::None }
        };
        pushes.push(quote! {
            key.push(::settings_types::schema::KeySpec {
                path: ::settings_types::schema::KeyPath(#path.to_owned()),
                kind: #kind,
                default: ::settings_types::schema::to_value(&default.#field_ident),
                label: ::settings_types::schema::Label(#label.to_owned()),
                help: ::settings_types::schema::Help(#help.to_owned()),
                page: #page,
                section: ::settings_types::schema::Section(#section.to_owned()),
                exposure: #exposure,
                labels: ::std::default::Default::default(),
                agent: #agent_setting,
                unavailable: ::std::default::Default::default(),
                groups: ::std::default::Default::default(),
                places: ::std::default::Default::default(),
                note: #note,
                control: #control,
                ends: #ends,
            });
        });
    }

    let footnotes = container
        .footnotes
        .iter()
        .map(|FootnoteAttr { section, text }| {
            quote! {
                ::settings_types::schema::Footnote {
                    page: #page,
                    section: ::settings_types::schema::Section(#section.to_owned()),
                    text: ::settings_types::schema::FootnoteText(#text.to_owned()),
                }
            }
        });
    Ok(quote! {
        #(#guards)*
        impl ::settings_types::schema::SettingsSchema for #ident {
            fn schema() -> ::settings_types::schema::Schema {
                let default = <Self as ::core::default::Default>::default();
                let mut key: ::std::vec::Vec<::settings_types::schema::KeySpec> = ::std::vec::Vec::new();
                #(#pushes)*
                ::settings_types::schema::Schema {
                    app: ::settings_types::schema::AppId(#app.to_owned()),
                    file: ::settings_types::schema::FilePath(#file.to_owned()),
                    version: 1,
                    foreign: ::settings_types::schema::ForeignTables::default(),
                    pages: ::std::vec::Vec::new(),
                    key,
                    footnotes: ::std::vec![#(#footnotes),*],
                }
            }
        }
    })
}

/// A compile-time assertion that `path` is not under a never-agent-settable prefix
/// (`settings_types::schema::AGENT_NEVER_SETTABLE`). The derive cannot read that list itself
/// (`settings-types` depends on this crate), so the check is a `const` evaluated in the
/// consumer's build: a violation is a compile error naming the key.
fn never_settable_guard(path: &str) -> TokenStream {
    let message = format!(
        "settings key `{path}` is under a never-agent-settable prefix \
         (settings_types::schema::AGENT_NEVER_SETTABLE): remove `agent_settable`"
    );
    quote! {
        const _: () = ::core::assert!(
            !::settings_types::schema::is_never_settable(#path),
            #message
        );
    }
}

/// The `KeyKind` expression for one field, from its shape, type and unit.
fn kind_expr(shape: Shape, ty: &syn::Type, unit: Option<&str>) -> TokenStream {
    match shape {
        Shape::Bounded { min, max } => {
            let unit = if let Some(unit) = unit {
                quote! { ::std::option::Option::Some(#unit.to_owned()) }
            } else {
                quote! { ::std::option::Option::None }
            };
            quote! { ::settings_types::schema::KeyKind::Bounded { min: #min, max: #max, unit: #unit } }
        }
        Shape::Toggle => quote! { ::settings_types::schema::KeyKind::Toggle },
        Shape::Text => quote! { ::settings_types::schema::KeyKind::Text },
        Shape::Provided { capability } => quote! {
            ::settings_types::schema::KeyKind::Provided { capability: #capability.to_owned() }
        },
        Shape::Colour => quote! { ::settings_types::schema::KeyKind::Colour },
        Shape::TextList => quote! {
            ::settings_types::schema::KeyKind::List(::std::boxed::Box::new(
                ::settings_types::schema::KeyKind::Text
            ))
        },
        Shape::List(inner) => {
            quote! { <#inner as ::settings_types::schema::ListElement>::list_kind() }
        }
        Shape::EnumType => quote! { ::settings_types::schema::kind_of::<#ty>() },
    }
}

#[cfg(test)]
mod tests {
    //! The compile errors a malformed struct gets, checked on the expansion itself: there is
    //! no `trybuild` in this workspace's lockfile to drive a UI test instead.

    use super::expand;

    fn expanded(source: &str) -> syn::Result<proc_macro2::TokenStream> {
        let input: syn::DeriveInput =
            syn::parse_str(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        let syn::Data::Struct(data) = &input.data else {
            panic!("{source}: not a struct");
        };
        expand(&input, data)
    }

    const HEAD: &str = "#[settings(file = \"a/s.toml\", domain = \"d\", page = Page::Dock)]";

    #[test]
    fn a_number_without_a_range_is_missing_range_naming_the_field() {
        const CASES: &[(&str, &str)] = &[
            ("u8", "size"),
            ("Px", "gap_px"),
            ("settings_types::Ms", "delay_ms"),
        ];
        for (ty, field) in CASES {
            let source = format!("{HEAD} struct S {{ #[settings(label = \"L\")] {field}: {ty} }}");
            let err = expanded(&source).err().map(|e| e.to_string());
            let want = format!("MissingRange {{ field: {field} }}");
            assert!(
                err.as_deref().is_some_and(|e| e.starts_with(&want)),
                "{ty}: {err:?}"
            );
        }
    }

    #[test]
    fn text_fields_are_text_by_type_or_attribute() {
        const CASES: &[&str] = &[
            "#[settings(label = \"L\")] f: String",
            "#[settings(label = \"L\")] f: std::path::PathBuf",
            "#[settings(label = \"L\")] f: Cow<'static, str>",
            "#[settings(label = \"L\", text)] f: SoundTheme",
        ];
        for field in CASES {
            let source = format!("{HEAD} struct S {{ {field} }}");
            let tokens = expanded(&source).unwrap_or_else(|e| panic!("{field}: {e}"));
            let text = tokens.to_string();
            assert!(text.contains("KeyKind :: Text"), "{field}: {text}");
        }
    }

    #[test]
    fn a_provided_field_is_a_provided_key_naming_its_capability() {
        let source = format!(
            "{HEAD} struct S {{ #[settings(label = \"L\", provided = \"look\")] f: Provided }}"
        );
        let text = expanded(&source)
            .unwrap_or_else(|e| panic!("{e}"))
            .to_string();
        assert!(text.contains("KeyKind :: Provided"), "{text}");
        assert!(text.contains("\"look\""), "{text}");
    }

    #[test]
    fn a_vec_is_a_list_of_text_or_whatever_its_element_says() {
        const CASES: &[(&str, &str)] = &[
            ("Vec<String>", "KeyKind :: List"),
            ("Vec<Row>", "ListElement > :: list_kind"),
        ];
        for (ty, want) in CASES {
            let source = format!("{HEAD} struct S {{ #[settings(label = \"L\")] f: {ty} }}");
            let text = expanded(&source)
                .unwrap_or_else(|e| panic!("{ty}: {e}"))
                .to_string();
            assert!(text.contains(want), "{ty}: {text}");
        }
    }

    #[test]
    fn agent_settable_emits_a_compile_time_never_settable_guard() {
        const CASES: &[(&str, &str)] = &[("cua", "cua.f"), ("agent", "agent.f"), ("d", "d.f")];
        for (domain, path) in CASES {
            let source = format!(
                "#[settings(file = \"a/s.toml\", domain = \"{domain}\", page = Page::Dock)] \
                 struct S {{ #[settings(label = \"L\", agent_settable)] f: String }}"
            );
            let text = expanded(&source)
                .unwrap_or_else(|e| panic!("{e}"))
                .to_string();
            assert!(
                text.contains(&format!("is_never_settable (\"{path}\")")),
                "{domain}: {text}"
            );
            assert!(
                text.contains("const _ : () = :: core :: assert !"),
                "{domain}: {text}"
            );
        }
    }

    #[test]
    fn a_hands_off_field_emits_no_guard() {
        let source = format!("{HEAD} struct S {{ #[settings(label = \"L\")] f: String }}");
        let text = expanded(&source)
            .unwrap_or_else(|e| panic!("{e}"))
            .to_string();
        assert!(!text.contains("is_never_settable"), "{text}");
    }

    #[test]
    fn a_named_control_is_emitted_on_the_key_and_none_otherwise() {
        const CASES: &[(&str, &str)] = &[
            (
                "#[settings(label = \"L\", control = \"picture\")]",
                "ControlName (\"picture\"",
            ),
            (
                "#[settings(label = \"L\")]",
                "control : :: std :: option :: Option :: None",
            ),
        ];
        for (attr, want) in CASES {
            let source = format!("{HEAD} struct S {{ {attr} f: String }}");
            let text = expanded(&source)
                .unwrap_or_else(|e| panic!("{attr}: {e}"))
                .to_string();
            assert!(text.contains(want), "{attr}: {text}");
        }
    }

    #[test]
    fn a_number_with_a_range_is_bounded() {
        let source = format!(
            "{HEAD} struct S {{ #[settings(label = \"L\", range = \"1..=9\", unit = \"px\")] f: Px }}"
        );
        let text = expanded(&source)
            .unwrap_or_else(|e| panic!("{e}"))
            .to_string();
        assert!(text.contains("KeyKind :: Bounded"), "{text}");
        assert!(text.contains("min : 1i64"), "{text}");
    }
}
