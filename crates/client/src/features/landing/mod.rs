//! Landing page feature for the CheenHub web client.

pub(crate) mod components;
pub(crate) mod data;
mod desktop;
mod native;
mod pages;
mod web;

pub(crate) use native::{
    LandingRoute, public_home_label, public_home_route, public_landing_available,
};
