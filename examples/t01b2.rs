extern crate csv;
extern crate ndarray;
extern crate ndarray_csv;
extern crate num_traits;
extern crate serde;
extern crate stopwatch;

use std::borrow::BorrowMut;
use std::cmp::{max, min};
use std::f64;
use std::io;

use ndarray::{array, stack};
use stopwatch::Stopwatch;

use quantaxis_rs::indicators::{
    BollingerBands, EfficiencyRatio, ExponentialMovingAverage, FastStochastic, Maximum, Minimum,
    MoneyFlowIndex, MovingAverage, MovingAverageConvergenceDivergence, OnBalanceVolume,
    RateOfChange, RelativeStrengthIndex, SimpleMovingAverage, SlowStochastic, StandardDeviation,
    TrueRange, HHV, LLV,
};
use quantaxis_rs::qaaccount::QA_Account;
use quantaxis_rs::qaposition::QA_Postions;
use quantaxis_rs::{
    indicators, qaaccount, qadata, qafetch, qaindicator, qaposition, transaction, Next,
};

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
    let priceoffset = 2;
    let lossP = 1.3;
    let K1: usize = 20;
    let K2: usize = 20;
    let n1: usize = 30;
    let mut bar_id = 0;
    let mut count1 = 0;
    let mut HAE: f64 = 0 as f64;
    let mut LAE: f64 = 0 as f64;
    let TrailingStart1 = 90.0;
    let TrailingStop1 = 10.0;
    let mut acc = qaaccount::QA_Account::new(
        "RustT01B2_RBL8",
        "test",
        "admin",
        1000000.0,
        false,
        "backtest",
    );
    acc.init_h("RBL8");
    let mut llv_i = LLV::new(K1 as u32).unwrap();
    let mut hhv_i = HHV::new(K2 as u32).unwrap();
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
        let ind_llv = llv_i.next(bar.low);
        let ind_hhv = hhv_i.next(bar.high);
        let ind_ma = ma.next(bar.open);
        let crossOver = bar.high > hhv_i.cached[K1 - 2] && lastbar.high < hhv_i.cached[K1 - 2];

        let crossUnder = bar.low < llv_i.cached[K2 - 2] && lastbar.low > llv_i.cached[K2 - 2];

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
                HAE = lastbar.high;
                LAE = lastbar.low;
            } else if bar_id - count1 > 1 {
                HAE = compare_max(HAE, lastbar.high);
                LAE = compare_min(LAE, lastbar.low);
            }
        }

        if long_pos == 0.0 && short_pos == 0.0 && hour_i32 < 21 && hour_i32 >= 9 {
            if crossOver && cond1 {
                println!("BUY OPEN");
                acc.buy_open(
                    bar.code.as_ref(),
                    90.0,
                    bar.datetime.as_ref(),
                    compare_max(bar.open, hhv_i.cached[K1 - 2]),
                );
                count1 = bar_id;
                HAE = 0.0;
                LAE = 0.0;
            } else if crossUnder && cond2 {
                println!("SELL OPEN");
                acc.sell_open(
                    bar.code.as_ref(),
                    90.0,
                    bar.datetime.as_ref(),
                    compare_min(bar.open, llv_i.cached[K2 - 2]),
                );
                count1 = bar_id;
                HAE = 0.0;
                LAE = 0.0;
            }
        }
        if long_pos > 0.0 && short_pos == 0.0 {
            //println!("当前多单持仓");

            let mut stopLine: f64 = acc.get_open_price_long(code) * (100.0 - lossP) / 100.0;
            if HAE >= (acc.get_open_price_long(code) * (1.0 + TrailingStart1 / 1000.0))
                && bar_id - count1 >= 1
            {
                //println!("CHANGE STOPLINE");
                stopLine = (HAE * (1.0 - TrailingStop1 / 1000.0));
            }

            if (crossUnder && cond2) {
                //println!("CORSSUNDER_SELLCLOSE");
                println!("SELL CLOSE");
                acc.sell_close(
                    code,
                    90.0,
                    bar.datetime.as_ref(),
                    compare_min(bar.open, llv_i.cached[K2 - 2]),
                );
            } else if (bar.low < stopLine) {
                //println!("LOW UNDER_SELLCLOSE");
                println!("SELL CLOSE FORCE");
                acc.sell_close(
                    code,
                    90.0,
                    bar.datetime.as_ref(),
                    compare_min(bar.open, stopLine),
                );
            }
        }
        if (short_pos > 0.0 && long_pos == 0.0) {
            //println!("当前空单持仓 {:#?}", acc.get_position_short(code));
            let mut stopLine: f64 = acc.get_open_price_short(code) * (100.0 + lossP) / 100.0;

            if (LAE >= (acc.get_open_price_short(code) * (1.0 - TrailingStart1 / 1000.0) as f64)
                && bar_id - count1 >= 1)
            {
                stopLine = (LAE * (1.0 + TrailingStop1 / 1000.0));
            }
            if crossOver && cond1 {
                println!("BUY CLOSE");
                acc.buy_close(
                    code,
                    90.0,
                    bar.datetime.as_ref(),
                    compare_max(bar.open, hhv_i.cached[K1 - 2]),
                );
            } else if (bar.high >= stopLine) {
                println!("BUY CLOSE Force");
                acc.buy_close(
                    code,
                    90.0,
                    bar.datetime.as_ref(),
                    compare_max(bar.open, stopLine),
                );
            }
        }
        if (short_pos == 0.0 && long_pos == 0.0) {
            count1 = bar_id;
            HAE = 0.0;
            LAE = 0.0;
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
    acc.to_csv();
    println!("It took {0:.8} ms", sw.elapsed_ms());
}
