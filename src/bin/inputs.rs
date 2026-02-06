use csv::{Reader, StringRecord};

fn main() {
    let mut args = std::env::args();
    let _ = args.next().unwrap();
    read_csv("input_ages.csv");
}

fn percent_in_age_range(input_ages: &[u32], filter: u32) -> f64 {
    let count = input_ages.iter().filter(|&age| *age <= filter).count();
    count as f64 / input_ages.len() as f64
}

fn average(percentages: &[f64]) -> f64 {
    percentages.iter().sum::<f64>() / percentages.len() as f64
}

fn read_csv(path: &str) {
    let mut rdr =
        Reader::from_path(path).expect("could not find `ages.csv`. have you ran `generate.rs`?");
    let mut total_5 = Vec::new();
    let mut total_10 = Vec::new();
    let mut total_50 = Vec::new();
    let mut total_100 = Vec::new();
    for result in rdr.records().skip(1) {
        let record: StringRecord = result.unwrap();
        let block = record[0].parse::<u32>().unwrap();
        let ages = record[1].parse::<String>().unwrap();
        if ages.is_empty() {
            continue;
        }
        let ages = ages
            .split("|")
            .map(|age| age.parse::<u32>().unwrap())
            .collect::<Vec<u32>>();
        let percent_are_5 = percent_in_age_range(&ages, 5);
        let percent_are_10 = percent_in_age_range(&ages, 10);
        let percent_are_50 = percent_in_age_range(&ages, 50);
        let percent_are_100 = percent_in_age_range(&ages, 100);
        println!(
            "block {block}: percentage in block lifespan; 5: {percent_are_5:.4}, 10: {percent_are_10:.4}, 50: {percent_are_50:.4}, 100: {percent_are_100:.4}"
        );
        total_5.push(percent_are_5);
        total_10.push(percent_are_10);
        total_50.push(percent_are_50);
        total_100.push(percent_are_100);
    }
    println!(" ");
    println!(">>> Summary >>>");
    println!(" ");
    println!("percentage 5 blocks or earlier {:.4}", average(&total_5));
    println!("percentage 10 blocks or earlier {:.4}", average(&total_10));
    println!("percentage 50 blocks or earlier {:.4}", average(&total_50));
    println!(
        "percentage 100 blocks or earlier {:.4}",
        average(&total_100)
    );
}
