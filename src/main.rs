extern crate scoped_threadpool;
extern crate hyper;
extern crate kuchiki;
extern crate redis;
extern crate num_cpus;

use hyper::Client;
use hyper::header::ContentType;
use kuchiki::traits::ParserExt;
use scoped_threadpool::Pool;
use redis::Commands;
use redis::Client as RedisClient;
use std::env;

const KEY: &'static str = "52633f4973cf845e55b18c8e22ab08d5";
const SEARCH_HOST: &'static str = "http://www.gainesvillemls.com";


fn main() {
    let http_client = &Client::new();
    let body = &format!("key={}&LM_MST_prop_fmtYNNT=1&LM_MST_prop_cdYYNT=1,9,10,11,12,13,14&LM_MST_mls_noYYNT=&LM_MST_list_prcYNNB=&LM_MST_list_prcYNNE=175000&LM_MST_prop_cdYNNL[]=9&LM_MST_sqft_nYNNB=&LM_MST_sqft_nYNNE=&LM_MST_yr_bltYNNB=&LM_MST_yr_bltYNNE=&LM_MST_bdrmsYNNB=3&LM_MST_bdrmsYNNE=&LM_MST_bathsYNNB=2&LM_MST_bathsYNNE=&LM_MST_hbathYNNB=&LM_MST_hbathYNNE=&LM_MST_countyYNCL[]=ALA&LM_MST_str_noY1CS=&LM_MST_str_namY1VZ=&LM_MST_remarksY1VZ=&openHouseStartDt_B=&openHouseStartDt_E=&ve_info=&ve_rgns=1&LM_MST_LATXX6I=&poi=&count=1&isLink=0&custom=", KEY);

    let redis_client = &redis::Client::open(&*env::var("REDIS_DSN").unwrap()).unwrap();

    let res = http_client.post(&format!("{}/gan/idx/search.php", SEARCH_HOST))
        .header(ContentType::form_url_encoded())
        .body(body)
        .send()
        .unwrap();

    let document = kuchiki::parse_html().from_http(res).unwrap();

    let cpus = num_cpus::get() * 4;
    let mut pool = Pool::new(cpus as u32);
    pool.scoped(|scope| {
        for listing in document.select("table.listings").unwrap() {
            let elem = listing.as_node();
            let text = elem.select("tr:nth-of-type(3)").unwrap().next().unwrap().text_contents();
            if text.to_lowercase().find("gainesville, fl") != None {
                let mls = elem.select("span.mls").unwrap().next().unwrap().text_contents();
                let price = elem.select("span.price").unwrap().next().unwrap().text_contents();
                scope.execute(move || {
                    check_block_and_parking(mls, price, http_client, redis_client);
                });
            }
        }
    });
}

fn check_block_and_parking(mls: String, price: String, http_client: &Client, redis_client: &RedisClient) {
    let redis_conn = redis_client.get_connection().unwrap();

    if redis_conn.hexists("mls", &*mls).unwrap() {
        return;
    }

    let _ : () = redis_conn.hset("mls", &*mls, &*price).unwrap();

    let res = http_client.post(&format!("{}/gan/idx/detail.php", SEARCH_HOST))
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
