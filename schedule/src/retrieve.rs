
//@todo Add the ability to do some basic filtering here, like passing through that we only 
//      want active opportunities or something.
pub fn opportunities() -> Vec<serde_json::Value> {
    let mut page = 0;
    let per_page = 20;
    let mut output = Vec::new();

    loop {
        page += 1;

        let result = currentrms::get_opportunities(&page, &per_page);
        match result {
            Err(err) => { println!("{}", err); break; }
            Ok(response) => {
                let opportunities_object = &response["opportunities"];
                if !opportunities_object.is_array() {
                    break;
                }

                // If there are no more opportunities then we're done.
                let opportunities = opportunities_object.as_array().unwrap();
                if opportunities.is_empty() {
                    break;
                }
        
                // Cloning all the objects found and push all these structures into a vector to
                // be returned.
                output.extend(opportunities.clone().into_iter());
            }
        };
    }

    output
}
