#![allow(incomplete_features)]
#![allow(async_fn_in_trait)]
#![feature(adt_const_params)]
#![feature(const_trait_impl)]
#![feature(try_trait_v2)]
#![feature(slice_take)]

pub mod web_ui;
pub mod cli;
pub mod utils;
pub mod client;
pub mod entities;
pub mod config;
pub mod storage;
pub mod error;
