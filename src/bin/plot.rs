use csv::Reader;
use plotters::prelude::*;
use std::collections::BTreeMap;

#[derive(serde::Deserialize)]
struct Row {
    age: u32,
    count: u64,
}

fn main() {
    let map = read_csv_to_btreemap("ages.csv");
    plot_age_count(map, "plot.png");
}

fn read_csv_to_btreemap(path: &str) -> BTreeMap<u32, u64> {
    let mut map = BTreeMap::new();
    let mut rdr =
        Reader::from_path(path).expect("could not find `ages.csv`. have you ran `generate.rs`?");
    for result in rdr.deserialize().skip(1) {
        let record: Row = result.unwrap();
        let age: u32 = record.age;
        let count: u64 = record.count;
        println!("Coin age {age}, number of occurrences {count}");
        map.insert(age, count);
    }
    map
}

fn plot_age_count(data: BTreeMap<u32, u64>, path: &str) {
    let root = BitMapBackend::new(path, (1024, 768)).into_drawing_area();
    root.fill(&WHITE).unwrap();

    let max_age = *data.keys().max().unwrap_or(&1);
    let max_count = *data.values().max().unwrap_or(&1);

    let mut chart = ChartBuilder::on(&root)
        .caption("UTXO Age Distribution", ("sans-serif", 30))
        .margin(20)
        .x_label_area_size(50)
        .y_label_area_size(80)
        .build_cartesian_2d(0u32..max_age, (1u64..max_count * 2).log_scale())
        .unwrap();

    chart
        .configure_mesh()
        .x_desc("Age (blocks)")
        .y_desc("Count")
        .y_label_formatter(&|y| {
            if *y >= 1_000_000 {
                format!("{}M", y / 1_000_000)
            } else if *y >= 1_000 {
                format!("{}K", y / 1_000)
            } else {
                format!("{}", y)
            }
        })
        .draw()
        .unwrap();

    chart
        .draw_series(LineSeries::new(
            data.iter().map(|(&age, &count)| (age, count.max(1))),
            BLUE.stroke_width(2),
        ))
        .unwrap()
        .label("UTXOs")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE.stroke_width(2)));

    chart
        .configure_series_labels()
        .background_style(WHITE.mix(0.8))
        .border_style(BLACK)
        .draw()
        .unwrap();

    root.present().unwrap();
}
