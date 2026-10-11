#![no_main]
use libfuzzer_sys::fuzz_target;
use ranterra_engine::{session,ngap,protocol,auth,model::Snssai};
fuzz_target!(|data: &[u8]| {
    if data.len()>65536 {return;}
    let _=protocol::inspect(data);
    let _=session::nas(data.to_vec());
    let _=session::unwrap_n1(data,true);
    let _=session::selection(data);
    let _=ngap::unwrap(data,101);
    let _=session::decode_setup(data,101,2,&Snssai::default());
    let _=session::response_teid(data,101);
    let _=session::validate_setup_failure(data,101);
    let _=session::validate_release_response(data,101);
    let _=session::decode_gtpu(data,2);
    let _=session::echo_reply(data,[10,45,0,1]);
    let _=auth::resynchronize(101,&[0;16],data,1);
});
