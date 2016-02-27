extern crate hyper;
extern crate kuchiki;
extern crate regex;

use std::sync::Arc;
use std::thread;
use hyper::Client;
use hyper::header::ContentType;
use kuchiki::traits::ParserExt;
use regex::Regex;

const KEY: &'static str = "52633f4973cf845e55b18c8e22ab08d5";
const SEARCH_HOST: &'static str = "http://www.gainesvillemls.com";


fn main() {
    let client = Arc::new(Client::new());
    let body = &format!("key={}&LM_MST_prop_fmtYNNT=1&LM_MST_prop_cdYYNT=1,9,10,11,12,13,14&LM_MST_mls_noYYNT=&LM_MST_list_prcYNNB=&LM_MST_list_prcYNNE=175000&LM_MST_prop_cdYNNL[]=9&LM_MST_sqft_nYNNB=&LM_MST_sqft_nYNNE=&LM_MST_yr_bltYNNB=&LM_MST_yr_bltYNNE=&LM_MST_bdrmsYNNB=3&LM_MST_bdrmsYNNE=&LM_MST_bathsYNNB=2&LM_MST_bathsYNNE=&LM_MST_hbathYNNB=&LM_MST_hbathYNNE=&LM_MST_countyYNCL[]=ALA&LM_MST_str_noY1CS=&LM_MST_str_namY1VZ=&LM_MST_remarksY1VZ=&openHouseStartDt_B=&openHouseStartDt_E=&ve_info=&ve_rgns=1&LM_MST_LATXX6I=&poi=&count=1&isLink=0&custom=", KEY);

    let res = client.post(&format!("{}/gan/idx/search.php", SEARCH_HOST))
        .header(ContentType::form_url_encoded())
        .body(body)
        .send()
        .unwrap();

    let document = kuchiki::parse_html().from_http(res).unwrap();

    let address_re = Regex::new(r"(?i)gainesville, fl").unwrap();
    let mut threads = vec![];
    for listings in document.select("table.listings").unwrap() {
        let elem = listings.as_node();
        let text = elem.select("tr:nth-of-type(3)").unwrap().next().unwrap().text_contents();

        if address_re.is_match(&text) {
            let mls = elem.select("span.mls").unwrap().next().unwrap().text_contents();
            let mls_client = client.clone();
            threads.push(thread::spawn(move || {
                check_block_and_parking(mls, mls_client);
            }));
        }
    }
    for thread in threads {
        let _ = thread.join();
    }
}

fn check_block_and_parking(mls: String, client: Arc<Client>) {
    let res = client.post(&format!("{}/gan/idx/detail.php", SEARCH_HOST))
        .header(ContentType::form_url_encoded())
        .body(&format!("key={}&mls={}&gallery=false&custom=", KEY, mls))
        .send()
        .unwrap();

    let document = kuchiki::parse_html().from_http(res).unwrap();
    let mut has_parking = true;
    let mut has_block = false;

    for details in document.select("table.wide label.bold").unwrap() {
        if "Parking:" == details.text_contents() && details.as_node().parent().unwrap().select("span").unwrap().next().unwrap().text_contents().to_lowercase().find("no garage") != None {
            has_parking = false;
        }
        if "Construction-exterior:" == details.text_contents() && details.as_node().parent().unwrap().select("span").unwrap().next().unwrap().text_contents().to_lowercase().find("block") != None {
            has_block = true;
        }
    }

    if has_parking && has_block {
        println!("{}/gan/idx/index.php?key={}&mls={}", SEARCH_HOST, KEY, mls);
    }
}
