use l2_handler::start_negotiation;
use l2_handler::handle_negotiation;
use l2_handler::parse_args;
use rocket::{launch, routes};

#[launch]
fn rocket() -> _ {
  parse_args();

  rocket::build().mount("/", routes![handle_negotiation, start_negotiation])
}