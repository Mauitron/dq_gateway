//#############################################################################################
//#                                 IMPORTANT INFORMATION                                     #
//#############################################################################################
//#   The codebase is at the moment synchronus. This should be amended when we have a working #
//#   prototype. at the moment, if i am not being too doom and gloom,                         #
//#   somewhere around 70%+ of the time used by this approach would likely                    #
//#   be on just waiting for things.                                                          #
//#############################################################################################

#![cfg_attr(rustfmt, rustfmt_skip)]
//##############################################################\\
//# -----------------------Dependencies----------------------- #\\
//#  If planning to add any dependencies (crates) be sure      #\\
//#  that you reflect on the costs of doing so. Is the time    #\\
//#  gained, worth layerl of abstractions or being dependent   #\\
//# ---------------------------------------------------------- #\\
//##############################################################\\
/////////////////////////////////\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\


use std::io::Read;
use std::time::{Duration, Instant};
use std::io::{self};
use std::net::{SocketAddr, TcpStream};
use std::collections::VecDeque;


//-------------IMPORTS---------------\\
pub mod the_teltonica_protocol;
pub mod binary_parser;
pub mod the_connector;
pub mod gate_guard;
pub mod gate_state;
pub mod pipeline;
pub mod testing;

//-----------------------------------\\



//-------------EXPORTS---------------\\
pub use the_teltonica_protocol::*;
pub use the_connector::*;
pub use binary_parser::*;
pub use gate_state::*;
pub use gate_guard::*;
pub use pipeline::*;
pub use testing::*;
//------------------------------------\\


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
