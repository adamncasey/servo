/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use quote;
use std::borrow::Cow;
use syn;
use synstructure;

pub fn derive(input: syn::DeriveInput) -> quote::Tokens {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let mut where_clause = where_clause.clone();
    for param in &input.generics.ty_params {
        where_clause.predicates.push(where_predicate(syn::Ty::Path(None, param.ident.clone().into())))
    }

    let variants = variants(&input);
    let mut match_body = quote!();
    match_body.append_all(variants.iter().map(|variant| {
        let name = match input.body {
            syn::Body::Struct(_) => Cow::Borrowed(&input.ident),
            syn::Body::Enum(_) => {
                Cow::Owned(syn::Ident::from(format!("{}::{}", input.ident, variant.ident)))
            },
        };
        let (this_pattern, this_info) = synstructure::match_pattern(
            &name,
            &variant.data,
            &synstructure::BindOpts::with_prefix(
                synstructure::BindStyle::Ref,
                "this".to_owned(),
            ),
        );
        let (other_pattern, other_info) = synstructure::match_pattern(
            &name,
            &variant.data,
            &synstructure::BindOpts::with_prefix(
                synstructure::BindStyle::Ref,
                "other".to_owned(),
            ),
        );
        let sum = if this_info.is_empty() {
            quote! { ::values::distance::SquaredDistance::Value(0.) }
        } else {
            let mut sum = quote!();
            sum.append_separated(this_info.iter().zip(&other_info).map(|(this, other)| {
                where_clause.predicates.push(where_predicate(this.field.ty.clone()));
                quote! {
                    ::values::distance::ComputeSquaredDistance::compute_squared_distance(#this, #other)?
                }
            }), "+");
            sum
        };
        quote! {
            (&#this_pattern, &#other_pattern) => {
                Ok(#sum)
            }
        }
    }));

    if variants.len() > 1 {
        match_body = quote! { #match_body, _ => Err(()), };
    }

    quote! {
        impl #impl_generics ::values::distance::ComputeSquaredDistance for #name #ty_generics #where_clause {
            #[allow(unused_variables, unused_imports)]
            #[inline]
            fn compute_squared_distance(
                &self,
                other: &Self,
            ) -> Result<::values::distance::SquaredDistance, ()> {
                match (self, other) {
                    #match_body
                }
            }
        }
    }
}

fn variants(input: &syn::DeriveInput) -> Cow<[syn::Variant]> {
    match input.body {
        syn::Body::Enum(ref variants) => (&**variants).into(),
        syn::Body::Struct(ref data) => {
            vec![syn::Variant {
                ident: input.ident.clone(),
                attrs: input.attrs.clone(),
                data: data.clone(),
                discriminant: None,
            }].into()
        },
    }
}

fn where_predicate(ty: syn::Ty) -> syn::WherePredicate {
    syn::WherePredicate::BoundPredicate(
        syn::WhereBoundPredicate {
            bound_lifetimes: vec![],
            bounded_ty: ty,
            bounds: vec![syn::TyParamBound::Trait(
                syn::PolyTraitRef {
                    bound_lifetimes: vec![],
                    trait_ref: syn::Path {
                        global: true,
                        segments: vec![
                            "values".into(),
                            "distance".into(),
                            "ComputeSquaredDistance".into(),
                        ],
                    },
                },
                syn::TraitBoundModifier::None,
            )],
        },
    )
}
