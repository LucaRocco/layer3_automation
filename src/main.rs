use l2_handler::start_negotiation;
use l2_handler::handle_negotiation;
use l2_handler::parse_args;
use rocket::{launch, routes};

#[launch]
fn rocket() -> _ {
  simple_logger::SimpleLogger::new().env().init().unwrap();
  
  parse_args();

  rocket::build().mount("/", routes![handle_negotiation, start_negotiation])
}