#![cfg_attr(rustfmt, rustfmt_skip)]
// use core::fmt;

pub mod binary_parser;
pub mod the_connector;
pub mod gate_guard;
pub mod the_teltonica_protocol;


//-------------------------------------------------------------------
//|                         CONSTANTS                               |\
//-------------------------------------------------------------------\
//    The smallest possible size of a AVL package                 //|\
      pub const SMALLEST_AVL_SIZE: usize = 45;                    //|\
//    The maximum possible size of AVL record from FM6XXX devices //|\
      pub const MAX_AVL_RECORD_SIZE_FM6XXX: usize = 255;          //|\
//    The maximum possible size of AVL packet from FM6XXX devices //|\
      pub const MAX_AVL_PACKET_SIZE_FM6XXX: usize = 512;          //|\
//    The maximum possible size  packet of any devices            //|\
      pub const LARGEST_AVL_SIZE: usize = 1280;                   //|\
//------------------------------------------------------------------|\
//-------------------------------------------------------------------\
