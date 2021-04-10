use chrono::{NaiveDate};
use std::collections::{HashMap};
use std::fmt;
use std::fs;

#[derive(Debug)]
struct SensorRecord<T> {
    pub timestamp: T,
    pub temperature: f32,
    pub humidity: f32,
    pub dew_point: f32,
    pub vpd: f32,
}

impl SensorRecord<NaiveDate> {
    fn from_csv_record(record: csv::StringRecord) -> Self {
        fn parse_date_time(datetime: &str) -> NaiveDate {
            let date_parts: Vec<&str> = datetime.split(" ").collect();
            let date_vec: Vec<&str> = date_parts[0].split("-").collect();

            return NaiveDate::from_ymd(
                date_vec[0].parse::<i32>().unwrap(),
                date_vec[1].parse::<u32>().unwrap(),
                date_vec[2].parse::<u32>().unwrap(),
            );
        }

        return SensorRecord {
            timestamp: parse_date_time(&record[0]),
            temperature: record[1].parse::<f32>().unwrap(),
            humidity: record[2].parse::<f32>().unwrap(),
            dew_point: record[3].parse::<f32>().unwrap(),
            vpd: record[4].parse::<f32>().unwrap(),
        };
    }
}

struct DaySummaries<T>(Vec<DaySummaryStats<T>>);

impl fmt::Display for DaySummaries<NaiveDate> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut output = String::new();
        if let Some(first_day_summary) = self.0.first() {
            if let Some(last_day_summary) = self.0.last() {
                output += format!("{} records for date range {} - {}", self.0.len(), first_day_summary.date, last_day_summary.date).as_str();
            }
        } else {
            output += format!("length: {}, date range: <na> - <na>", self.0.len()).as_str();
        }

        write!(f, "{}", output)
    }
}

impl fmt::Display for DaySummaryStats<NaiveDate> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}\n{}\n{}\n{}\n{}\ngdd: {}\n",
            self.date,
            self.temperature_stats,
            self.humidity_stats,
            self.dew_point_stats,
            self.vpd_stats,
            self.gdd,
        )
    }
}

impl DaySummaries<NaiveDate> {
    // Assumes Records are pre-sorted in a chronologically ascending order.
    fn add_record(&mut self, record: &SensorRecord<NaiveDate>) {
        match self.0.last_mut() {
            Some(day_summary_stats) => {
                if day_summary_stats.date == record.timestamp {
                    day_summary_stats.calc_temperature_stats(record);
                    day_summary_stats.calc_humidity_stats(record);
                    day_summary_stats.calc_dew_point_stats(record);
                    day_summary_stats.calc_vpd_stats(record);
                    day_summary_stats.calc_growing_degrees_day();
                } else {
                    self.0.push(DaySummaryStats::from_record(record));
                }
            },
            None => {
                self.0.push(DaySummaryStats::from_record(record));
            }
        }
    }
}

#[derive(Debug, Clone)]
struct TemperatureStats {
    pub max_temperature: f32,
    pub min_temperature: f32,
    pub mean_temperature: f32,
    pub median_temperature: f32,
    pub temperature_entries: Vec<f32>,
    pub temperature_sum: f32,
}

impl fmt::Display for TemperatureStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "temp: mean: {} max: {} min: {}",
            self.mean_temperature,
            self.max_temperature,
            self.min_temperature,
        )
    }
}

#[derive(Debug, Clone)]
struct HumidityStats {
    pub max_humidity: f32,
    pub min_humidity: f32,
    pub mean_humidity: f32,
    pub median_humidity: f32,
    pub humidity_entries: Vec<f32>,
    pub humidity_sum: f32,
}

impl fmt::Display for HumidityStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "humidity: mean: {} max: {} min: {}",
            self.mean_humidity,
            self.max_humidity,
            self.min_humidity,
        )
    }
}

#[derive(Debug, Clone)]
struct DewPointStats {
    pub max_dew_point: f32,
    pub min_dew_point: f32,
    pub mean_dew_point: f32,
    pub median_dew_point: f32,
    pub dew_point_entries: Vec<f32>,
    pub dew_point_sum: f32,
}

impl fmt::Display for DewPointStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "dew_point: mean: {} max: {} min: {}",
            self.mean_dew_point,
            self.max_dew_point,
            self.min_dew_point,
        )
    }
}

#[derive(Debug, Clone)]
struct VPDStats {
    pub max_vpd: f32,
    pub min_vpd: f32,
    pub mean_vpd: f32,
    pub median_vpd: f32,
    pub vpd_entries: Vec<f32>,
    pub vpd_sum: f32,
}

impl fmt::Display for VPDStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "vpd: mean: {} max: {} min: {}",
            self.mean_vpd,
            self.max_vpd,
            self.min_vpd,
        )
    }
}

#[derive(Debug, Clone)]
struct DaySummaryStats<T> {
    pub date: T,
    pub temperature_stats: TemperatureStats,
    pub humidity_stats: HumidityStats,
    pub dew_point_stats: DewPointStats,
    pub vpd_stats: VPDStats,
    pub gdd: f32, // gdd is growing degree days, a measure of heat units per day a crop receives over its lifetime relative to the minimum base temperature required for growth of that crop. e.g. corn's base temperature is 50°F. Given a day whose average temperature was 75°F, the crop would have grown by 1.5 gdd (75°F - 50°F = 15 gdd).
}

// TODO: This should be configurable as either an env var or a cli arg.
static GDD_THRESHOLD : f32 = 65.0;
impl DaySummaryStats<NaiveDate> {
    fn from_record(record: &SensorRecord<NaiveDate>) -> Self {
        let temperature_stats = TemperatureStats {
            max_temperature: record.temperature,
            min_temperature: record.temperature,
            mean_temperature: record.temperature,
            median_temperature: record.temperature,
            temperature_entries: vec![record.temperature],
            temperature_sum: record.temperature,
        };
        let humidity_stats = HumidityStats {
            max_humidity: record.humidity,
            min_humidity: record.humidity,
            mean_humidity: record.humidity,
            median_humidity: record.humidity,
            humidity_entries: vec![record.humidity],
            humidity_sum: record.humidity,
        };
        let dew_point_stats = DewPointStats {
            max_dew_point: record.dew_point,
            min_dew_point: record.dew_point,
            mean_dew_point: record.dew_point,
            median_dew_point: record.dew_point,
            dew_point_entries: vec![record.dew_point],
            dew_point_sum: record.dew_point,
        };
        let vpd_stats = VPDStats {
            max_vpd: record.vpd,
            min_vpd: record.vpd,
            mean_vpd: record.vpd,
            median_vpd: record.vpd,
            vpd_entries: vec![record.vpd],
            vpd_sum: record.vpd,
        };
        return DaySummaryStats {
            date: record.timestamp,
            temperature_stats: temperature_stats,
            humidity_stats: humidity_stats,
            dew_point_stats: dew_point_stats,
            vpd_stats: vpd_stats,
            gdd: record.temperature - GDD_THRESHOLD,
        };
    }

    fn calc_temperature_stats(&mut self, record: &SensorRecord<NaiveDate>) {
        // Add the temperature to the accumulated sum
        self.temperature_stats.temperature_sum += record.temperature;

        // First add the record to the temperature stat entries.
        self.temperature_stats.temperature_entries.push(record.temperature);

        // Find the max temperature.
        self.temperature_stats.max_temperature = *self.temperature_stats.temperature_entries.iter().max_by(|x, y| x.partial_cmp(&y).unwrap()).unwrap();

        // Find the min temperature.
        self.temperature_stats.min_temperature = *self.temperature_stats.temperature_entries.iter().min_by(|x, y| x.partial_cmp(&y).unwrap()).unwrap();

        // Find the median temperature.
        let median_index = self.temperature_stats.temperature_entries.len() / 2;
        self.temperature_stats.median_temperature = self.temperature_stats.temperature_entries[median_index];

        // Find the mean temperature.
        let mean_denominator = self.temperature_stats.temperature_entries.len() as f32;
        self.temperature_stats.mean_temperature = self.temperature_stats.temperature_sum / mean_denominator;
    }

    fn calc_humidity_stats(&mut self, record: &SensorRecord<NaiveDate>) {
        // Add the humidity to the accumulated sum
        self.humidity_stats.humidity_sum += record.humidity;

        // First add the record to the humidity stat entries.
        self.humidity_stats.humidity_entries.push(record.humidity);

        // Find the max humidity.
        self.humidity_stats.max_humidity = *self.humidity_stats.humidity_entries.iter().max_by(|x, y| x.partial_cmp(&y).unwrap()).unwrap();

        // Find the min humidity.
        self.humidity_stats.min_humidity = *self.humidity_stats.humidity_entries.iter().min_by(|x, y| x.partial_cmp(&y).unwrap()).unwrap();

        // Find the median humidity.
        let median_index = self.humidity_stats.humidity_entries.len() / 2;
        self.humidity_stats.median_humidity = self.humidity_stats.humidity_entries[median_index];

        // Find the mean humidity.
        let mean_denominator = self.humidity_stats.humidity_entries.len() as f32;
        self.humidity_stats.mean_humidity = self.humidity_stats.humidity_sum / mean_denominator;
    }

    fn calc_dew_point_stats(&mut self, record: &SensorRecord<NaiveDate>) {
        // Add the humidity to the accumulated sum
        self.dew_point_stats.dew_point_sum += record.dew_point;

        // First add the record to the humidity stat entries.
        self.dew_point_stats.dew_point_entries.push(record.dew_point);

        // Find the max humidity.
        self.dew_point_stats.max_dew_point = *self.dew_point_stats.dew_point_entries.iter().max_by(|x, y| x.partial_cmp(&y).unwrap()).unwrap();

        // Find the min humidity.
        self.dew_point_stats.min_dew_point = *self.dew_point_stats.dew_point_entries.iter().min_by(|x, y| x.partial_cmp(&y).unwrap()).unwrap();

        // Find the median humidity.
        let median_index = self.dew_point_stats.dew_point_entries.len() / 2;
        self.dew_point_stats.median_dew_point = self.dew_point_stats.dew_point_entries[median_index];

        // Find the mean humidity.
        let mean_denominator = self.dew_point_stats.dew_point_entries.len() as f32;
        self.dew_point_stats.mean_dew_point = self.dew_point_stats.dew_point_sum / mean_denominator;
    }

    fn calc_vpd_stats(&mut self, record: &SensorRecord<NaiveDate>) {
        // Add the humidity to the accumulated sum
        self.vpd_stats.vpd_sum += record.vpd;

        // First add the record to the humidity stat entries.
        self.vpd_stats.vpd_entries.push(record.vpd);

        // Find the max humidity.
        self.vpd_stats.max_vpd = *self.vpd_stats.vpd_entries.iter().max_by(|x, y| x.partial_cmp(&y).unwrap()).unwrap();

        // Find the min humidity.
        self.vpd_stats.min_vpd = *self.vpd_stats.vpd_entries.iter().min_by(|x, y| x.partial_cmp(&y).unwrap()).unwrap();

        // Find the median humidity.
        let median_index = self.vpd_stats.vpd_entries.len() / 2;
        self.vpd_stats.median_vpd = self.vpd_stats.vpd_entries[median_index];

        // Find the mean humidity.
        let mean_denominator = self.vpd_stats.vpd_entries.len() as f32;
        self.vpd_stats.mean_vpd = self.vpd_stats.vpd_sum / mean_denominator;
    }

    fn calc_growing_degrees_day(&mut self) {
        // TODO: calculate GDD for day and night. This calculation currently uses 1 value for a 24 hour time period.
        self.gdd = self.temperature_stats.mean_temperature - GDD_THRESHOLD;
        // If degree day is long or short, the calculation is slightly different:
        // if degree_day.short() {
        //    gdd.growing_degrees_day = (day_summary.temperature_stats.mean_day_temperature + day_summary.temperature_stats.mean_night_temperature) / 2.0;
        // } else {
        //    gdd.growing_degrees_day = ((day_summary.temperature_stats.mean_day_temperature * 0.67) + (day_summary.temperature_stats.mean_night_temperature * 0.33)) / 2.0;
        // }
    }
}

fn main() -> Result<(), csv::Error> {
    let sensor_data = fs::read_to_string("data/example.csv").expect("Error reading csv file.");
    let mut sensor_reader = csv::Reader::from_reader(sensor_data.as_bytes());

    let event_data = fs::read_to_string("data/events.csv").expect("Error reading csv file.");
    let mut event_reader = csv::Reader::from_reader(event_data.as_bytes());

    let mut writer = csv::Writer::from_path("data/out_example.csv")?;
    writer.write_record(&["date", "avg temp", "max temp", "min temp", "avg humidity", "max humidity", "min humidity", "avg dewpoint", "avg vpd", "gdd", "event"])?;

    let mut event_summaries = HashMap::new();
    for record in event_reader.records() {
        let record: csv::StringRecord = record?;
        let date_parts: Vec<&str> = record[0].split(" ").collect();
        let date_vec: Vec<&str> = date_parts[0].split("-").collect();
        let date = NaiveDate::from_ymd(
                date_vec[0].parse::<i32>().unwrap(),
                date_vec[1].parse::<u32>().unwrap(),
                date_vec[2].parse::<u32>().unwrap(),
            );
        let event = record[1].parse::<String>().unwrap();
        event_summaries.insert(date.to_string(), event);
    }

    let mut day_summaries = DaySummaries(Vec::new());
    for record in sensor_reader.records() {
        let record: csv::StringRecord = record?;
        let record_entry = SensorRecord::from_csv_record(record);
        day_summaries.add_record(&record_entry);
    };


    println!("day summaries: {}", day_summaries);
    let mut total_gdd = 0.0;
    for day_summary in &day_summaries.0 {
        let mut event = String::new();
        if event_summaries.contains_key(&day_summary.date.to_string()) {
            event = event_summaries.get(&day_summary.date.to_string()).unwrap().to_string();
        }

        total_gdd += day_summary.gdd;

        writer.write_record(&[
            day_summary.date.to_string(),
            day_summary.temperature_stats.mean_temperature.to_string(),
            day_summary.temperature_stats.max_temperature.to_string(),
            day_summary.temperature_stats.min_temperature.to_string(),
            day_summary.humidity_stats.mean_humidity.to_string(),
            day_summary.humidity_stats.max_humidity.to_string(),
            day_summary.humidity_stats.min_humidity.to_string(),
            day_summary.dew_point_stats.mean_dew_point.to_string(),
            day_summary.vpd_stats.mean_vpd.to_string(),
            total_gdd.to_string(),
            event.to_string(),
        ])?;
    };

    writer.flush()?;

    Ok(())
}
