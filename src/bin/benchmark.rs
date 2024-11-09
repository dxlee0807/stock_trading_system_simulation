extern crate bma_benchmark;
mod bursa;
use std::collections::HashMap;
use crate::bursa::{Order, Stock,UserRule,OrderType};

use crossbeam_channel::{unbounded,Sender};
use std::sync::{Mutex,Arc,mpsc::channel};

use bma_benchmark::{benchmark_stage, staged_benchmark_print_for};
use std::hint::black_box;
use bursa::{read_stock_file_to_vector,read_user_rule_file_to_hashmap,
    simplified_select_and_send_stock,simplified_update_stock_price,
    simplified_process_user_order};

fn main(){
    #[benchmark_stage(i=10000,name="read_stock_file")]
    fn benchmark_read_stock_file_to_vector(path:&str){
        let _a = read_stock_file_to_vector(path);
    }
    #[benchmark_stage(i=10000,name="read_user_rule_file")]
    fn benchmark_read_user_rule_file_to_hashmap(path:&str){
        let _a = read_user_rule_file_to_hashmap(path);
    }


    let (stock_selector_tx, stock_selector_rx) = unbounded();
    let (stock_updater_tx, stock_updater_rx) = channel();
    // let (broadcaster_tx, broadcaster_rx) = unbounded();
    let stock_selector_tx_clone_clone:crossbeam_channel::Sender<Order> = stock_selector_tx.clone();
    let stock_arc_used_by_selector = Arc::new(Mutex::new(read_stock_file_to_vector("./src/data/stocks.txt"))).clone();
    let stock_arc_used_by_updater = stock_arc_used_by_selector.clone();
    let user_rule_arc_used_by_broker = Arc::new(Mutex::new(read_user_rule_file_to_hashmap("./src/data/user_rules.txt"))).clone();


    #[benchmark_stage(i=10000,name="select_and_send_stock")]
    fn benchmark_simplified_select_and_send_stock(stock_arc_used_by_selector:Arc<Mutex<Vec<Stock>>>,stock_selector_tx:crossbeam_channel::Sender<Order>){
        simplified_select_and_send_stock(stock_arc_used_by_selector.clone(),stock_selector_tx.clone());
    }

    #[benchmark_stage(i=10000,name="update_stock_start")]
    fn benchmark_simplified_update_stock_start(stock_arc_used_by_updater:Arc<Mutex<Vec<Stock>>>,sender:std::sync::mpsc::Sender<usize>){
        let order = Order{stock_id:1,user_rule:UserRule{user_id:-1,order:OrderType::Start,price:0.0,volume:0}};
        simplified_update_stock_price(order,stock_arc_used_by_updater.clone(),sender.clone());
    }

    #[benchmark_stage(i=10000,name="update_stock_bid")]
    fn benchmark_simplified_update_stock_bid(stock_arc_used_by_updater:Arc<Mutex<Vec<Stock>>>,sender:std::sync::mpsc::Sender<usize>){
        let order = Order{stock_id:1,user_rule:UserRule{user_id:-1,order:OrderType::Bid,price:0.0,volume:100}};
        simplified_update_stock_price(order,stock_arc_used_by_updater.clone(),sender.clone());
    }
    
    #[benchmark_stage(i=10000,name="update_stock_ask")]
    fn benchmark_simplified_update_stock_ask(stock_arc_used_by_updater:Arc<Mutex<Vec<Stock>>>,sender:std::sync::mpsc::Sender<usize>){
        let order = Order{stock_id:1,user_rule:UserRule{user_id:-1,order:OrderType::Ask,price:0.0,volume:100}};
        simplified_update_stock_price(order,stock_arc_used_by_updater.clone(),sender.clone());
    }

    #[benchmark_stage(i=10000,name="process_user_order")]
    fn benchmark_simplified_process_user_order(stock_arc_used_by_broker:Arc<Mutex<Vec<Stock>>>,
        user_rule_arc_used_by_broker:Arc<Mutex<HashMap<String,Vec<UserRule>>>>,
        stock_selector_tx_clone_clone:Sender<Order>){
        simplified_process_user_order(stock_arc_used_by_broker.clone(),user_rule_arc_used_by_broker.clone(),stock_selector_tx_clone_clone.clone());
    }

    // start benchmark
    benchmark_read_stock_file_to_vector("./src/data/stocks.txt");
    benchmark_read_user_rule_file_to_hashmap("./src/data/user_rules.txt");
    benchmark_simplified_select_and_send_stock(stock_arc_used_by_selector,stock_selector_tx);
    benchmark_simplified_update_stock_start(stock_arc_used_by_updater.clone(),stock_updater_tx.clone());
    benchmark_simplified_update_stock_bid(stock_arc_used_by_updater.clone(),stock_updater_tx.clone());
    benchmark_simplified_update_stock_ask(stock_arc_used_by_updater.clone(),stock_updater_tx.clone());
    benchmark_simplified_process_user_order(stock_arc_used_by_updater.clone(),
        user_rule_arc_used_by_broker,
        stock_selector_tx_clone_clone);
    staged_benchmark_print_for!("read_user_rule_file");
}