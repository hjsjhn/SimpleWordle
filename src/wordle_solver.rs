mod builtin_words;

use console;
use text_io::read;
use std::{
    collections::{HashMap, HashSet},
    io::{self, BufRead, BufReader, Write},
};

use crate::builtin_words::{FINAL, ACCEPTABLE};

#[derive(Debug)]
struct ArgsErr<'a>(&'a str);
impl std::fmt::Display for ArgsErr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "ArgsError: {}", self.0)
    }
}
impl std::error::Error for ArgsErr<'_> {}

enum Color {
    Red,
    Green,
    Blue,
    Yellow,
    Nothing,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum AlphStatus {
    //the status of alphabet
    Right,
    PosWrong,
    TooMany,
    Unknown,
}

impl AlphStatus {
    fn parse1(&self) -> u32 {
        //parse for comparing priority
        match &self {
            AlphStatus::Right => 3,
            AlphStatus::PosWrong => 2,
            AlphStatus::TooMany => 1,
            AlphStatus::Unknown => 0,
        }
    }

    fn parse2(&self) -> Color {
        //parse for getting color code (None,R,G,B,Y)->(0,1,2,3,4)
        match &self {
            AlphStatus::Right => Color::Green,
            AlphStatus::PosWrong => Color::Yellow,
            AlphStatus::TooMany => Color::Red,
            AlphStatus::Unknown => Color::Nothing,
        }
    }

    fn parse3(&self) -> String {
        //parse for getting status G,Y,R,X
        match &self {
            AlphStatus::Right => "G".to_string(),    //Green
            AlphStatus::PosWrong => "Y".to_string(), //Yellow
            AlphStatus::TooMany => "R".to_string(),  //Red
            AlphStatus::Unknown => "X".to_string(),  //Unknown
        }
    }
}

struct WordleSolver {
    final_set: Vec<String>,
    acceptable_set: Vec<String>,
}

impl WordleSolver {
    const ALPHABET: &'static str = "abcdefghijklmnopqrstuvwxyz";

    fn printall(pln: bool, words: &str, tty: bool, bold: Option<bool>, color: Option<Color>) {
        if tty {
            // color: 0->nothing 1->red 2->green 3->blue 4->yellow
            let bd: bool = bold.unwrap_or(false);
            let mut col: Color = color.unwrap_or(Color::Nothing);
            let mut stl = console::style(words.to_string());
            if bd {
                stl = stl.bold();
            }
            match col {
                Color::Red => stl = stl.red(),
                Color::Green => stl = stl.green(),
                Color::Blue => stl = stl.blue(),
                Color::Yellow => stl = stl.yellow(),
                _ => {}
            };
            match pln {
                true => println!("{}", stl),
                false => print!("{}", stl),
            };
            io::stdout().flush().unwrap();
        }
    }

    fn println(words: &str, tty: bool, bold: Option<bool>, color: Option<Color>) {
        WordleSolver::printall(true, words, tty, bold, color);
    }

    fn print(words: &str, tty: bool, bold: Option<bool>, color: Option<Color>) {
        WordleSolver::printall(false, words, tty, bold, color);
    }

    fn read() -> String {
        let mut key_word = String::new();
        io::stdin().read_line(&mut key_word);
        key_word = key_word.trim().to_string();
        key_word
    }

    fn new(
        final_set: Vec<String>,
        acceptable_set: Vec<String>,
    ) -> WordleSolver {
        WordleSolver {
            final_set: final_set,
            acceptable_set: acceptable_set,
        }
    }

    fn check_possible(
        input: &str,
        status: &HashMap<char, AlphStatus>,
        green_word: &Vec<char>,
        numbers: &mut HashMap<char, i32>,
        forbid: &mut HashMap<char, Vec<u32>>,
    ) -> bool {
        let word: String = input.to_string();
        let mut tmp: usize = 0;
        let mut ninput: Vec<char> = vec![];
        let mut cnt_map: HashMap<char, i32> = HashMap::new();
        for c in word.chars() {
            *cnt_map.entry(c).or_insert(0) += 1;
        }
        for c in word.chars() {
            if forbid.entry(c).or_insert(vec![]).contains(&(tmp as u32)) {
                return false;
            }
            let cnt = (*numbers).entry(c).or_insert(-1);
            if *cnt != -1 && *cnt_map.get(&c).unwrap() != *cnt {
                return false;
            }
            if green_word[tmp] != '\0' && c != green_word[tmp] {
                return false;
            } else {
                ninput.push(c);
            }
            tmp += 1;
        }
        for c in WordleSolver::ALPHABET.chars() {
            if *status.get(&c).unwrap() == AlphStatus::PosWrong {
                if !ninput.contains(&c) {
                    return false;
                }
            }
        }
        true
    }

    fn recommend_word(
        &self,
        status: &HashMap<char, AlphStatus>,
        green_word: &Vec<char>,
        numbers: &mut HashMap<char, i32>,
        forbid: &mut HashMap<char, Vec<u32>>,
        input_word: &mut String
    ) {
        let mut cnt: u32 = 0;
        let mut possible_word: Vec<String> = vec![];
        WordleSolver::println(
            "Possibly correct words:",
            true,
            Some(true),
            Some(Color::Blue),
        );
        for word in &self.acceptable_set {
            if cnt == 5 {
                print!("...");
                cnt += 1;
            }
            if WordleSolver::check_possible(&word, status, green_word, numbers, forbid) {
                if cnt < 5 {
                    print!(
                        "{}{}",
                        match cnt {
                            0 => "",
                            _ => " ",
                        },
                        &word.to_uppercase()
                    );
                }
                possible_word.push(word.to_string());
                cnt += 1;
            }
        }
        println!("");

        // a slow way to calculate shanon information enrtopy
        let total: u32 = possible_word.len() as u32;
        let mut words: HashMap<String, f32> = HashMap::new();
        for word in &possible_word {
            let mut cnt: Vec<u32> = vec![0; 243]; //243=3^5 which present all the states
            for input in &possible_word {
                if word != input {
                    let mut map = HashMap::new();
                    let mut curstatus = vec![0; 5];
                    let mut tmp: usize = 0;
                    for (&c1, &c2) in word
                        .chars()
                        .collect::<Vec<char>>()
                        .iter()
                        .zip(input.chars().collect::<Vec<char>>().iter())
                    {
                        let count = map.entry(c1).or_insert(0);
                        if c1 == c2 {
                            curstatus[tmp] = 2;
                        } else {
                            *count += 1;
                        }
                        tmp += 1;
                    }
                    tmp = 0;
                    for c in input.chars() {
                        let count = map.entry(c).or_insert(0);
                        if *count > 0 && curstatus[tmp] != 2 {
                            curstatus[tmp] = 1;
                            *count -= 1;
                        }
                        tmp += 1;
                    }
                    let mut st: u32 = 0;
                    let mut base: u32 = 1;
                    for i in 0..5 {
                        st += base * curstatus[i];
                        base = base * 3;
                    }
                    cnt[st as usize] += 1;
                }
            }
            let mut ans: f32 = 0.0;
            for i in 0..243 {
                if cnt[i] != 0 {
                    ans -= (cnt[i] as f32) / (total as f32)
                        * ((cnt[i] as f32) / (total as f32)).log2();
                }
            }
            words.insert(word.to_string(), ans);
        }
        let mut count_vec: Vec<(&String, &f32)> = words.iter().collect();
        count_vec.sort_by(|a, b| a.0.cmp(b.0));
        count_vec.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
        WordleSolver::println(&format!("I recommend you use: {}", count_vec[0].0.to_uppercase()).to_string(), true, Some(true), Some(Color::Blue));
        *input_word = count_vec[0].0.to_lowercase();
    }

    fn play(&self, input_word: &mut String) {
        let mut cnt: usize = 0;
        let mut status = HashMap::new();
        for c in WordleSolver::ALPHABET.chars() {
            status.insert(c, AlphStatus::Unknown);
        }
        let mut curstatus: Vec<AlphStatus> = vec![AlphStatus::TooMany; 5];
        let mut green_word: Vec<char> = vec!['\0'; 5];
        let mut numbers: HashMap<char, i32> = HashMap::new();
        let mut forbid: HashMap<char, Vec<u32>> = HashMap::new();

        loop {
            cnt = cnt + 1;
            if cnt != 1 { self.recommend_word(&status, &green_word, &mut numbers, &mut forbid, input_word); }

            //update status of the word
            let mut cnt_map = HashMap::new();

            WordleSolver::println("Please input the status of the last word:", true, Some(true), Some(Color::Blue));
            let mut input_status: String = read!();
            input_status = input_status.to_uppercase();
            if !input_status.contains('R') && !input_status.contains('Y') { println!("SUCCESS!"); return; }

            curstatus = vec![AlphStatus::TooMany; 5];
            let mut tmp: usize = 0;
            for (&c1, &c2) in input_status
                .chars()
                .collect::<Vec<char>>()
                .iter()
                .zip(input_word.chars().collect::<Vec<char>>().iter())
            {
                if c1 == 'G' {
                    curstatus[tmp] = AlphStatus::Right;
                    *cnt_map.entry(c2).or_insert(0) += 1;
                    green_word[tmp] = c2.clone();
                } else if c1 == 'Y' {
                    curstatus[tmp] = AlphStatus::PosWrong;
                    *cnt_map.entry(c2).or_insert(0) += 1;
                    (*forbid.entry(c2).or_insert(vec![])).push(tmp as u32);
                }
                tmp += 1;
            }
            tmp = 0;
            for (&c1, &c2) in input_status
                .chars()
                .collect::<Vec<char>>()
                .iter()
                .zip(input_word.chars().collect::<Vec<char>>().iter())
            {
                if c1 != 'G' && c1 != 'Y' {
                    curstatus[tmp] = AlphStatus::TooMany;
                    *numbers.entry(c2).or_insert(-1) = *cnt_map.entry(c2).or_insert(0);
                }
                tmp += 1;
            }
            tmp = 0;
            for c in input_word.chars() {
                let oldstatus: &AlphStatus = status.get(&c).unwrap();
                let newstatus: &AlphStatus = &curstatus[tmp];
                if oldstatus.parse1() < newstatus.parse1() {
                    status.insert(c, *newstatus);
                }
                tmp += 1;
            } 
        }
    }
}

fn main() {
    let words: Vec<String> = vec!["salet".to_string(), "reast".to_string(), "crate".to_string(), "trace".to_string(), "slate".to_string(), "crane".to_string()];
    println!("Welcome to wordle solver.");
    println!("Pick a word from below and start your game:");
    for (index, word) in words.iter().enumerate() {
        print!("{}{}", match index { 0 => "", _ => ", "}, word.to_uppercase());
    } 
    println!("");
    let mut input_word: String = read!();
    input_word = input_word.to_lowercase();
    loop {
        if words.contains(&input_word) { break; }
        else {
            println!("Make sure that the word you input is one of the words above.");
            input_word = read!();
        }
    }
    let wordle_solver = WordleSolver::new(builtin_words::FINAL.iter().map(|s| s.to_string()).collect(), builtin_words::ACCEPTABLE.iter().map(|s| s.to_string()).collect());
    wordle_solver.play(& mut input_word);
}
