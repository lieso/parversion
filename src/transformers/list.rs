extern crate regex;

use crate::models;
use regex::Regex;

#[derive(Debug)]
struct Values {
    key: String,
    values: Vec<(String, usize)>,
}

pub fn transform_document_to_list(document: String, parser: &models::list::ListParser) -> models::list::List {
    log::trace!("In transform_document_to_list");


    let mut all_values: Vec<Values> = Vec::new();



    for (key, pattern) in parser.patterns.iter() {
        log::debug!("key: {}, pattern: {}", key, pattern);



        // TODO: investigate why this is necessary
        let fixed_pattern = &pattern.replace("\\\\", "\\");
        log::debug!("fixed_pattern: {}", fixed_pattern);




        let mut keyValues: Vec<(String, usize)> = Vec::new();

        if let Ok(regex) = Regex::new(fixed_pattern) {

            let matches: Vec<(&str, usize, usize)> = regex
                .captures_iter(&document)
                .filter_map(|cap| {
                    cap.get(1).map(|mat| (mat.as_str(), mat.start(), mat.end()))
                })
                .collect();

            for (value, _start, end) in matches {
                keyValues.push((value.to_string(), end));
            }

            let values = Values {
                key: key.to_string(),
                values: keyValues,
            };

            all_values.push(values);

        } else {
            log::error!("Regex pattern is invalid");
        }
    }





    all_values.sort_by_key(|v| v.values.len());






    let mut list_items = Vec::new();











    let first_data_set = &all_values[0];


    for i in 0..first_data_set.values.len() {

        let mut list_item = models::list::ListItem::new();


        let mut previous_index: usize = 0;

        for j in 0..all_values.len() {


            let current = &all_values[j];
            let key = &current.key;
            let values = &current.values[i];

            if j == 0 {
                previous_index = values.1;

                list_item.insert(key.clone(), values.0.clone());
            } else {
                let (nearest_index, nearest_value) = get_nearest_neighbour(current.values.clone(), previous_index);

                list_item.insert(key.clone(), nearest_value.clone());

                //previous_index = nearest_index.clone();
            }

        }


        list_items.push(list_item);
    }











    let list = models::list::List {
        items: list_items,
    };


    return list;
}

fn get_nearest_neighbour(values: Vec<(String, usize)>, previous_index: usize) -> (usize, String) {
    log::trace!("In get_nearest_neighbour");

    let mut optimal: usize = usize::MAX;
    let mut optimal_index: usize = 0;

    for (index, (_text, doc_index)) in values.iter().enumerate() {
        let distance = abs_diff(*doc_index, previous_index);

        if distance < optimal {
            optimal = distance;
            optimal_index = index;
        }
    }

    if optimal_index < values.len() {
        return (optimal_index, values[optimal_index].0.clone());
    } else {
        return (optimal_index, String::new());
    }
}

fn abs_diff(a: usize, b: usize) -> usize {
    if a > b {
        a - b
    } else {
        b - a
    }
 }
