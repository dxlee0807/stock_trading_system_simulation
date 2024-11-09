use std::{env, fs, str::FromStr};
use strum_macros::{EnumString,Display};
use std::collections::HashMap;

use crossbeam_channel::{unbounded,Sender};
use std::{ops::Deref,thread,sync::{Mutex,Arc,mpsc::channel},time::Duration,mem::drop};
use scheduled_thread_pool::ScheduledThreadPool;

use rand::Rng;

#[derive(Debug,Clone)]
pub struct Stock{
    pub symbol:String,
    pub price:f32,
    pub volume:i32
}
impl Deref for Stock{
    type Target = f32;
    fn deref(&self) -> &Self::Target{
        &self.price
    }
}

#[derive(EnumString,Debug,Display,Clone)]
pub enum OrderType{
    Start,Bid,Ask
}

// Define a struct to represent user preferences
#[derive(Debug,Clone)]
pub struct UserRule{
    pub user_id:i32,
    pub order:OrderType,
    pub price:f32,
    pub volume:i32
}

#[derive(Debug,Clone)]
pub struct Order{
    pub stock_id: usize,
    pub user_rule:UserRule
}

pub fn read_stock_file_to_vector(path: &str) -> Vec<Stock>{
    // read stocks data into a Stock vector
    let mut stock_vector:Vec<Stock> = vec![];
    let stock_file_string = fs::read_to_string(path);
    match stock_file_string {
        Ok(stock_file_string)=>{
            let stock_lines = stock_file_string.lines();
            for stock in stock_lines{
                let stock_attrs:Vec<&str> = stock.split(",").collect();
                stock_vector.push(Stock{
                    symbol:String::from(stock_attrs[0]),
                    price:stock_attrs[1].parse::<f32>().unwrap(),
                    volume:stock_attrs[2].parse::<i32>().unwrap()
                });
            }
            return stock_vector
        },
        Err(e)=>{
            println!("Stock file not found: {}",e);
            return stock_vector
        }
    } 
}

pub fn read_user_rule_file_to_hashmap(path: &str) -> HashMap<String,Vec<UserRule>>{
    // read user rules data into a Stock vector
    // a {stock:vec![UserRule]} dictionary
    let mut stock_with_user_rules = HashMap::new();
    let user_rule_file_string = fs::read_to_string(path);
    match user_rule_file_string {
        Ok(user_rule_file_string)=>{
            let user_rule_lines = user_rule_file_string.lines();
            for line in user_rule_lines{
                let split_stock_with_user_rules:Vec<&str> = line.split(":").collect();
                let stock_symbol = String::from(split_stock_with_user_rules[0]);
                let user_rules = String::from(split_stock_with_user_rules[1]);
                let user_rule_line_vector:Vec<&str> = user_rules.split(";").collect();
                let mut user_rule_vector = vec![];
                for ur_line in user_rule_line_vector{
                    let user_rule_attrs:Vec<&str> = ur_line.split(",").collect();
                    user_rule_vector.push(UserRule{
                        user_id:user_rule_attrs[0].parse::<i32>().unwrap(),
                        order:OrderType::from_str(user_rule_attrs[1]).unwrap(),
                        price:user_rule_attrs[2].parse::<f32>().unwrap(),
                        volume:user_rule_attrs[3].parse::<i32>().unwrap()
                    });
                }
                stock_with_user_rules.insert(stock_symbol,user_rule_vector);
            }
            return stock_with_user_rules
        },
        Err(e)=>{
            println!("User Rule file not found: {}",e);
            return stock_with_user_rules
        }
    } 
}

pub fn select_and_send_stock(stock_arc_used_by_selector:Arc<Mutex<Vec<Stock>>>,stock_selector_tx:crossbeam_channel::Sender<Order>) {
    let mut stock_mutex = stock_arc_used_by_selector.lock().unwrap();
    let number_of_stocks = stock_mutex.len();
    for i in 0..number_of_stocks{
        let stock = &mut stock_mutex[i];
        println!("SELECTOR: {:?}",stock);
        stock_selector_tx.send(Order{stock_id:i,user_rule:UserRule{user_id:-1,order:OrderType::Start,price:0.0,volume:0}}).unwrap();
        thread::sleep(Duration::from_millis(100));
    }
}

pub fn simplified_select_and_send_stock(stock_arc_used_by_selector:Arc<Mutex<Vec<Stock>>>,stock_selector_tx:crossbeam_channel::Sender<Order>) {
    let mut stock_mutex = stock_arc_used_by_selector.lock().unwrap();
    let number_of_stocks = stock_mutex.len();
    for i in 0..number_of_stocks{
        let stock = &mut stock_mutex[i];
        stock_selector_tx.send(Order{stock_id:i,user_rule:UserRule{user_id:-1,order:OrderType::Start,price:0.0,volume:0}}).unwrap();
    }
}

pub fn update_stock_price(_i:i32,order:Order,stock_arc_used_by_updater:Arc<Mutex<Vec<Stock>>>,sender:std::sync::mpsc::Sender<usize>){
    let order_type:OrderType = order.user_rule.order;
    let selected_stock = order.stock_id;
    let stock = {
        let mut master_stocks = stock_arc_used_by_updater.lock().unwrap();
        let stock = &mut master_stocks[selected_stock];
        let mut new_price:f32 = match order_type{
            OrderType::Start=>stock.price * rand::thread_rng().gen_range(0.8..1.2),
            OrderType::Bid=>stock.price * rand::thread_rng().gen_range(1.0..1.2),
            OrderType::Ask=>stock.price * rand::thread_rng().gen_range(0.8..1.0)
        };
        new_price = (new_price*100.0).round()/100.0;
        let new_volume = match order_type{
            OrderType::Start=>stock.volume,
            OrderType::Bid=>stock.volume - order.user_rule.volume,
            OrderType::Ask=>stock.volume + order.user_rule.volume
        };
        let stock = Stock{
            symbol:String::from(stock.symbol.to_ascii_uppercase()),
            price:new_price,
            volume:new_volume
        };
        stock
    };
    let mut update_stocks = stock_arc_used_by_updater.lock().unwrap();
    update_stocks[selected_stock] = stock;
    sender.send(selected_stock).unwrap();
    match order_type{
        OrderType::Start=>{
            println!("\tStart: UPDATER {} AFTER: {:?}",_i,update_stocks[selected_stock]);
        },
        OrderType::Bid=>{
            println!("\tBid: UPDATER {} AFTER: {:?}",_i,update_stocks[selected_stock]);
        },
        OrderType::Ask=>{
            println!("\tAsk: UPDATER {} AFTER: {:?}",_i,update_stocks[selected_stock]);
        }
    }
    drop(update_stocks);
}

pub fn simplified_update_stock_price(order:Order,stock_arc_used_by_updater:Arc<Mutex<Vec<Stock>>>,sender:std::sync::mpsc::Sender<usize>){
    let order_type:OrderType = order.user_rule.order;
    let selected_stock = order.stock_id;
    let stock = {
        let mut master_stocks = stock_arc_used_by_updater.lock().unwrap();
        let stock = &mut master_stocks[selected_stock];
        let mut new_price:f32 = match order_type{
            OrderType::Start=>stock.price * rand::thread_rng().gen_range(0.8..1.2),
            OrderType::Bid=>stock.price * rand::thread_rng().gen_range(1.0..1.2),
            OrderType::Ask=>stock.price * rand::thread_rng().gen_range(0.8..1.0)
        };
        new_price = (new_price*100.0).round()/100.0;
        let new_volume = match order_type{
            OrderType::Start=>stock.volume,
            OrderType::Bid=>stock.volume - order.user_rule.volume,
            OrderType::Ask=>stock.volume + order.user_rule.volume
        };
        let stock = Stock{
            symbol:String::from(stock.symbol.to_ascii_uppercase()),
            price:new_price,
            volume:new_volume
        };
        stock
    };
    let mut update_stocks = stock_arc_used_by_updater.lock().unwrap();
    update_stocks[selected_stock] = stock;
    sender.send(selected_stock).unwrap();
    drop(update_stocks);
}

pub fn process_user_order(_i:i32,stock_arc_used_by_broker:Arc<Mutex<Vec<Stock>>>,
    user_rule_arc_used_by_broker:Arc<Mutex<HashMap<String,Vec<UserRule>>>>,
    broadcaster_rx_clone:crossbeam_channel::Receiver<usize>,
    stock_selector_tx_clone_clone:Sender<Order>){
    for updated_stock in broadcaster_rx_clone.iter() {
        // Process the updated stock information
        let stocks = stock_arc_used_by_broker.lock().unwrap();
        let stock = &stocks[updated_stock];
        let st_urs = user_rule_arc_used_by_broker.lock().unwrap();
        let urs = st_urs.deref().get(&stock.symbol).unwrap();
        // scan every user rule in the hashmap, if condition met then complete the exchange 
        for ur in urs{
            match ur.order{
                OrderType::Start=>{},
                OrderType::Bid=>{
                    // if stock price is lower/equal to bid price, buy
                    if stock.price<=ur.price{
                        // complete the buying process
                        println!("\t\t\tBROKER {}: user_{} BUY {} shares of {} with a price of {}",_i,ur.user_id,ur.volume,&stock.symbol,stock.price);
                        stock_selector_tx_clone_clone.send(Order{stock_id:updated_stock,user_rule:ur.clone()}).unwrap();
                    }
                },
                OrderType::Ask=>{
                    // if stock price is higher/equal to ask price, sell
                    if stock.price>=ur.price{
                        // complete the selling process
                        println!("\t\t\tBROKER {}: user_{} SELL {} shares of {} with a price of {}",_i,ur.user_id,ur.volume,&stock.symbol,stock.price);
                        stock_selector_tx_clone_clone.send(Order{stock_id:updated_stock,user_rule:ur.clone()}).unwrap();
                    }
                }
            }
        };
    }
}

pub fn simplified_process_user_order(stock_arc_used_by_broker:Arc<Mutex<Vec<Stock>>>,
    user_rule_arc_used_by_broker:Arc<Mutex<HashMap<String,Vec<UserRule>>>>,
    stock_selector_tx_clone_clone:Sender<Order>){
    // Process the updated stock information
    let stocks = stock_arc_used_by_broker.lock().unwrap();
    let updated_stock = 1;
    let stock = &stocks[updated_stock];
    let st_urs = user_rule_arc_used_by_broker.lock().unwrap();
    let urs = st_urs.deref().get(&stock.symbol).unwrap();
    // scan every user rule in the hashmap, if condition met then complete the exchange 
    for ur in urs{
        match ur.order{
            OrderType::Start=>{},
            OrderType::Bid=>{
                // if stock price is lower/equal to bid price, buy
                if stock.price<=ur.price{
                    // complete the buying process
                    stock_selector_tx_clone_clone.send(Order{stock_id:updated_stock,user_rule:ur.clone()}).unwrap();
                }
            },
            OrderType::Ask=>{
                // if stock price is higher/equal to ask price, sell
                if stock.price>=ur.price{
                    // complete the selling process
                    stock_selector_tx_clone_clone.send(Order{stock_id:updated_stock,user_rule:ur.clone()}).unwrap();
                }
            }
        }
    };
}

pub fn main(){
    env::set_var("RUST_BACKTRACE", "full");
    // read stocks data into a Stock vector
    let mut stock_vector = read_stock_file_to_vector("./src/data/stocks.txt");
    let stocks = Arc::new(Mutex::new(stock_vector));

    // read user rules data into a UserRule vector
    let mut stock_with_user_rules = read_user_rule_file_to_hashmap("./src/data/user_rules.txt");
    let user_rules = Arc::new(Mutex::new(stock_with_user_rules));

    println!("========== Trading Simulation Start! ==============");

    // create channels for communication between threads
    let (stock_selector_tx, stock_selector_rx) = unbounded();
    let (stock_updater_tx, stock_updater_rx) = channel();
    let (broadcaster_tx, broadcaster_rx) = unbounded();
    let stock_selector_tx_clone = stock_selector_tx.clone();

    // create scheduler
    let sched = ScheduledThreadPool::new(10);

    // 1 stock selector
    let stock_arc_used_by_selector = stocks.clone();
    thread::spawn(move ||{
        select_and_send_stock(stock_arc_used_by_selector,stock_selector_tx);
    });

    // 3 stock price updaters
    for _i in 1..4{
        let receiver = stock_selector_rx.clone();
        let sender = stock_updater_tx.clone();
        let stock_arc_used_by_updater = stocks.clone();
        sched.execute_at_fixed_rate(
            Duration::from_secs(5), 
            Duration::from_millis(500), 
            move||{
                let order:Order = receiver.recv().unwrap();
                update_stock_price(_i,order,stock_arc_used_by_updater.clone(),sender.clone());
            }
        );
    }
    // 1 broadcaster
    let stock_arc_used_by_broadcaster = stocks.clone();
    sched.execute_at_fixed_rate(
        Duration::from_secs(5), 
        Duration::from_millis(200), 
        move||{
            let stock_received_index = stock_updater_rx.recv().unwrap();
            let mut master_stocks = stock_arc_used_by_broadcaster.lock().unwrap();
            let stock = &mut master_stocks[stock_received_index];

            broadcaster_tx.send(stock_received_index).unwrap();
            println!("\t\tBROADCASTER: {:?}",stock);
    });

    // 5 brokers
    for _i in 1..6 {
        let stock_arc_used_by_broker = stocks.clone();
        let user_rule_arc_used_by_broker = user_rules.clone();
        let broadcaster_rx_clone = broadcaster_rx.clone();
        let stock_selector_tx_clone_clone = stock_selector_tx_clone.clone();
        thread::spawn(move || {
            process_user_order(_i, stock_arc_used_by_broker.clone(),
                user_rule_arc_used_by_broker.clone(),
                broadcaster_rx_clone.clone(),
                stock_selector_tx_clone_clone.clone());
        });
    }

    // a necessary loop to keep the stock simulation run continuously
    loop{}
}