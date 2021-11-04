extern crate csv;
extern crate ndarray;
extern crate ndarray_csv;
extern crate num_traits;
extern crate serde;
extern crate stopwatch;

use std::f64;
use std::io;

use stopwatch::Stopwatch;

use quantaxis_rs::indicators::{MovingAverage, HHV, LLV};
use quantaxis_rs::qaaccount::QA_Account;
use quantaxis_rs::{qaaccount, qafetch, Next};

trait FloatIterExt {
    fn float_min(&mut self) -> f64;
    fn float_max(&mut self) -> f64;
}

impl<T> FloatIterExt for T
where
    T: Iterator<Item = f64>,
{
    fn float_max(&mut self) -> f64 {
        self.fold(f64::NAN, f64::max)
    }

    fn float_min(&mut self) -> f64 {
        self.fold(f64::NAN, f64::min)
    }
}

fn compare_max(a: f64, b: f64) -> f64 {
    if a >= b {
        a
    } else {
        b
    }
}

fn compare_min(a: f64, b: f64) -> f64 {
    if a >= b {
        b
    } else {
        a
    }
}

pub fn backtest() -> QA_Account {
    let loss_p = 1.3;
    let k1: usize = 20;
    let k2: usize = 20;
    let n1: usize = 30;
    let mut bar_id = 0;
    let mut count1 = 0;
    let mut hae: f64 = 0 as f64;
    let mut lae: f64 = 0 as f64;
    let trailing_start1 = 90.0;
    let trailing_stop1 = 10.0;
    let mut acc = qaaccount::QA_Account::new(
        "RustT01B2_RBL8",
        "test",
        "admin",
        1000000.0,
        false,
        "backtest",
    );
    acc.init_h("RBL8");
    let mut llv_i = LLV::new(k1 as u32).unwrap();
    let mut hhv_i = HHV::new(k2 as u32).unwrap();
    let mut ma = MovingAverage::new(n1 as u32).unwrap();
    let mut rdr = csv::Reader::from_reader(io::stdin());
    let mut lastbar = qafetch::BAR {
        code: "".to_string(),
        datetime: "".to_string(),
        open: 0.0,
        high: 0.0,
        low: 0.0,
        close: 0.0,
        volume: 0.0,
    };
    for result in rdr.deserialize() {
        let bar: qafetch::BAR = result.unwrap();
        bar_id += 1;
        let hour = &bar.datetime[11..13];
        let hour_i32 = hour.parse::<i32>().unwrap();
        let _ind_llv = llv_i.next(bar.low);
        let _ind_hhv = hhv_i.next(bar.high);
        let _ind_ma = ma.next(bar.open);
        let cross_over = bar.high > hhv_i.cached[k1 - 2] && lastbar.high < hhv_i.cached[k1 - 2];

        let cross_under = bar.low < llv_i.cached[k2 - 2] && lastbar.low > llv_i.cached[k2 - 2];

        let cond1 = ma.cached[n1 - 2] > ma.cached[n1 - 3]
            && ma.cached[n1 - 3] > ma.cached[n1 - 4]
            && ma.cached[n1 - 4] > ma.cached[n1 - 5]
            && ma.cached[n1 - 5] > ma.cached[n1 - 6];

        let cond2 = ma.cached[n1 - 2] < ma.cached[n1 - 3]
            && ma.cached[n1 - 3] < ma.cached[n1 - 4]
            && ma.cached[n1 - 4] < ma.cached[n1 - 5]
            && ma.cached[n1 - 5] < ma.cached[n1 - 6];

        let code = bar.code.as_ref();

        let long_pos = acc.get_volume_long(code);
        let short_pos = acc.get_volume_short(code);
        if long_pos > 0.0 || short_pos > 0.0 {
            if bar_id - count1 == 1 {
                hae = lastbar.high;
                lae = lastbar.low;
            } else if bar_id - count1 > 1 {
                hae = compare_max(hae, lastbar.high);
                lae = compare_min(lae, lastbar.low);
            }
        }

        if long_pos == 0.0 && short_pos == 0.0 && hour_i32 < 21 && hour_i32 >= 9 {
            if cross_over && cond1 {
                println!("BUY OPEN");
                acc.buy_open(
                    bar.code.as_ref(),
                    90.0,
                    bar.datetime.as_ref(),
                    compare_max(bar.open, hhv_i.cached[k1 - 2]),
                )
                .unwrap();
                count1 = bar_id;
                hae = 0.0;
                lae = 0.0;
            } else if cross_under && cond2 {
                println!("SELL OPEN");
                acc.sell_open(
                    bar.code.as_ref(),
                    90.0,
                    bar.datetime.as_ref(),
                    compare_min(bar.open, llv_i.cached[k2 - 2]),
                )
                .unwrap();
                count1 = bar_id;
                hae = 0.0;
                lae = 0.0;
            }
        }
        if long_pos > 0.0 && short_pos == 0.0 {
            //println!("当前多单持仓");

            let mut stop_line: f64 = acc.get_open_price_long(code) * (100.0 - loss_p) / 100.0;
            if hae >= (acc.get_open_price_long(code) * (1.0 + trailing_start1 / 1000.0))
                && bar_id - count1 >= 1
            {
                //println!("CHANGE STOPLINE");
                stop_line = hae * (1.0 - trailing_stop1 / 1000.0);
            }

            if cross_under && cond2 {
                //println!("CORSSUNDER_SELLCLOSE");
                println!("SELL CLOSE");
                acc.sell_close(
                    code,
                    90.0,
                    bar.datetime.as_ref(),
                    compare_min(bar.open, llv_i.cached[k2 - 2]),
                )
                .unwrap();
            } else if bar.low < stop_line {
                //println!("LOW UNDER_SELLCLOSE");
                println!("SELL CLOSE FORCE");
                acc.sell_close(
                    code,
                    90.0,
                    bar.datetime.as_ref(),
                    compare_min(bar.open, stop_line),
                )
                .unwrap();
            }
        }
        if short_pos > 0.0 && long_pos == 0.0 {
            //println!("当前空单持仓 {:#?}", acc.get_position_short(code));
            let mut stop_line: f64 = acc.get_open_price_short(code) * (100.0 + loss_p) / 100.0;

            if lae >= acc.get_open_price_short(code) * (1.0 - trailing_start1 / 1000.0)
                && bar_id - count1 >= 1
            {
                stop_line = lae * (1.0 + trailing_stop1 / 1000.0);
            }
            if cross_over && cond1 {
                println!("BUY CLOSE");
                acc.buy_close(
                    code,
                    90.0,
                    bar.datetime.as_ref(),
                    compare_max(bar.open, hhv_i.cached[k1 - 2]),
                )
                .unwrap();
            } else if bar.high >= stop_line {
                println!("BUY CLOSE Force");
                acc.buy_close(
                    code,
                    90.0,
                    bar.datetime.as_ref(),
                    compare_max(bar.open, stop_line),
                )
                .unwrap();
            }
        }
        if short_pos == 0.0 && long_pos == 0.0 {
            count1 = bar_id;
            hae = 0.0;
            lae = 0.0;
        }

        lastbar = bar;
    }

    //qaaccount::QA_Account::history_table(&mut acc);

    acc
}

fn main() {
    let sw = Stopwatch::start_new();
    let acc = backtest();
    println!("LAST MONEY {:?}", acc.money);
    println!("{:?}", acc.cash);
    //println!("{:?}", acc.frozen);
    acc.to_csv().unwrap();
    println!("It took {0:.8} ms", sw.elapsed_ms());
}
