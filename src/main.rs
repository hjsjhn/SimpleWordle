mod builtin_words;

use clap::{App, Arg, ArgMatches};
use console;
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use serde_derive::{Deserialize, Serialize};
use serde_json;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{self, BufRead, BufReader, Write},
    path::Path,
};

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
struct Game {
    answer: String,
    guesses: Vec<String>,
}

impl Game {
    fn new() -> Game {
        Game {
            answer: "".to_string(),
            guesses: vec![],
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
struct State {
    total_rounds: u32,
    games: Vec<Game>,
}

impl State {
    fn new() -> State {
        State {
            total_rounds: 0,
            games: vec![],
        }
    }
}

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

struct Wordle {
    key_word: String,
    hard_mod: bool,
    stats: bool,
    seed: u64,
    tty: bool,
    final_set: Vec<String>,
    acceptable_set: Vec<String>,
}

impl Wordle {
    const SEED: u64 = 19260817998244353;
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
        Wordle::printall(true, words, tty, bold, color);
    }

    fn print(words: &str, tty: bool, bold: Option<bool>, color: Option<Color>) {
        Wordle::printall(false, words, tty, bold, color);
    }

    fn testout(words: &str, tty: bool) {
        if !tty {
            print!("{}", words.to_string());
        }
    }

    fn read() -> String {
        let mut key_word = String::new();
        io::stdin().read_line(&mut key_word);
        key_word = key_word.trim().to_string();
        key_word
    }

    fn new(
        key_word: String,
        hard_mod: bool,
        stats: bool,
        seed: u64,
        tty: bool,
        final_set: Vec<String>,
        acceptable_set: Vec<String>,
    ) -> Wordle {
        Wordle {
            key_word: key_word,
            hard_mod: hard_mod,
            stats: stats,
            seed: seed,
            tty: tty,
            final_set: final_set,
            acceptable_set: acceptable_set,
        }
    }

    fn trans_to_onum(cnt: usize) -> String {
        match cnt {
            1 => "1st".to_string(),
            2 => "2nd".to_string(),
            3 => "3rd".to_string(),
            4 => "4th".to_string(),
            5 => "5th".to_string(),
            6 => "6th".to_string(),
            _ => "Too Large".to_string(), //it won't occur
        }
    }

    fn check_hard_mod(
        &self,
        input_word: &str,
        curstatus: &Vec<AlphStatus>,
        status: &HashMap<char, AlphStatus>,
    ) -> bool {
        if !self.hard_mod {
            return true;
        }
        let input: String = input_word.to_string();
        let mut tmp: usize = 0;
        let mut ninput: Vec<char> = vec![];
        for (&c1, &c2) in self
            .key_word
            .chars()
            .collect::<Vec<char>>()
            .iter()
            .zip(input.chars().collect::<Vec<char>>().iter())
        {
            if curstatus[tmp] == AlphStatus::Right {
                if c1 != c2 {
                    return false;
                }
            } else {
                ninput.push(c2);
            }
            tmp += 1;
        }
        for c in Wordle::ALPHABET.chars() {
            if *status.get(&c).unwrap() == AlphStatus::PosWrong {
                if !ninput.contains(&c) {
                    return false;
                }
            }
        }
        true
    }

    fn check_word(
        &self,
        input_word: &str,
        curstatus: &Vec<AlphStatus>,
        status: &HashMap<char, AlphStatus>,
    ) -> bool {
        input_word.len() == 5
            && self.acceptable_set.contains(&input_word.to_string())
            && self.check_hard_mod(input_word, curstatus, status)
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
        for c in Wordle::ALPHABET.chars() {
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
    ) {
        let mut cnt: u32 = 0;
        let mut possible_word: Vec<String> = vec![];
        Wordle::println(
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
            if Wordle::check_possible(&word, status, green_word, numbers, forbid) {
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
        Wordle::println("I recommend you use:", true, Some(true), Some(Color::Blue));
        for (index, value) in count_vec.iter().enumerate() {
            if index > 4 {
                break;
            }
            print!(
                "{}{}({:.2} Bits)",
                match index {
                    0 => "",
                    _ => ", ",
                },
                value.0.to_uppercase(),
                value.1
            );
        }
        println!("");
    }

    fn play(&self, words_map: &mut HashMap<String, u32>) -> (u32, u32, Game) {
        let mut cnt: usize = 0;
        let mut win_tag: u32 = 0;
        let mut status = HashMap::new();
        for c in Wordle::ALPHABET.chars() {
            status.insert(c, AlphStatus::Unknown);
        }
        let mut curstatus: Vec<AlphStatus> = vec![AlphStatus::TooMany; 5];
        let mut green_word: Vec<char> = vec!['\0'; 5];
        let mut numbers: HashMap<char, i32> = HashMap::new();
        let mut forbid: HashMap<char, Vec<u32>> = HashMap::new();
        let mut game = Game::new();
        game.answer = self.key_word.to_string().to_uppercase();

        loop {
            cnt = cnt + 1;
            let mut input_word = String::new();
            input_word = "tares".to_string();
            if self.tty && cnt != 1 {
                self.recommend_word(&status, &green_word, &mut numbers, &mut forbid);
            }
            Wordle::print(
                &format!("Start Guessing({}): ", Wordle::trans_to_onum(cnt)).to_string(),
                self.tty,
                Some(true),
                Some(Color::Blue),
            );
            loop {
                input_word = Wordle::read();
                if self.check_word(&input_word, &curstatus, &status) {
                    break;
                } else {
                    Wordle::print(
                        "Key word format error or not in word list. Input again: ",
                        self.tty,
                        Some(false),
                        Some(Color::Red),
                    );
                    Wordle::testout("INVALID\n", self.tty);
                }
            }

            game.guesses.push(input_word.to_string().to_uppercase());
            *words_map.entry(input_word.to_string()).or_insert(0) += 1;

            //update status of the word
            let mut map = HashMap::new();
            let mut cnt_map = HashMap::new();
            curstatus = vec![AlphStatus::TooMany; 5];
            let mut tmp: usize = 0;
            for (&c1, &c2) in self
                .key_word
                .chars()
                .collect::<Vec<char>>()
                .iter()
                .zip(input_word.chars().collect::<Vec<char>>().iter())
            {
                let count = map.entry(c1).or_insert(0);
                if c1 == c2 {
                    curstatus[tmp] = AlphStatus::Right;
                    *cnt_map.entry(c2).or_insert(0) += 1;
                    green_word[tmp] = c1.clone();
                } else {
                    *count += 1;
                }
                tmp += 1;
            }
            tmp = 0;
            for c in input_word.chars() {
                let count = map.entry(c).or_insert(0);
                if *count > 0 && curstatus[tmp] != AlphStatus::Right {
                    curstatus[tmp] = AlphStatus::PosWrong;
                    *cnt_map.entry(c).or_insert(0) += 1;
                    (*forbid.entry(c).or_insert(vec![])).push(tmp as u32);
                    *count -= 1;
                }
                if curstatus[tmp] == AlphStatus::TooMany {
                    *numbers.entry(c).or_insert(-1) = *cnt_map.entry(c).or_insert(0);
                }
                tmp += 1;
            }

            //update stauts of the alphabet
            tmp = 0;
            for c in input_word.chars() {
                let oldstatus: &AlphStatus = status.get(&c).unwrap();
                let newstatus: &AlphStatus = &curstatus[tmp];
                if oldstatus.parse1() < newstatus.parse1() {
                    status.insert(c, *newstatus);
                }
                tmp += 1;
            }

            // println!("{:?}", curstatus);

            // print status for user
            tmp = 0;
            for c in input_word.chars() {
                Wordle::print(
                    &c.to_string(),
                    self.tty,
                    Some(false),
                    Some(curstatus[tmp].parse2()),
                );
                tmp += 1;
            }
            Wordle::println("", self.tty, None, None);
            for c in Wordle::ALPHABET.chars() {
                Wordle::print(
                    &c.to_string(),
                    self.tty,
                    Some(false),
                    Some(status.get(&c).unwrap().parse2()),
                );
            }
            Wordle::println("", self.tty, None, None);

            // print status for test
            for i in 0..5 {
                Wordle::testout(&curstatus[i].parse3(), self.tty);
            }
            Wordle::testout(" ", self.tty);
            for c in Wordle::ALPHABET.chars() {
                Wordle::testout(&status.get(&c).unwrap().parse3(), self.tty);
            }
            Wordle::testout("\n", self.tty);

            // judement
            if input_word == self.key_word {
                Wordle::println(
                    &format!("CORRECT, guess time: {}", cnt).to_string(),
                    self.tty,
                    Some(true),
                    Some(Color::Green),
                );
                Wordle::testout(&format!("CORRECT {}\n", cnt).to_string(), self.tty);
                win_tag = 1;
                break;
            }
            if cnt == 6 {
                Wordle::println(
                    "LOST, you failed too many times.",
                    self.tty,
                    Some(true),
                    Some(Color::Red),
                );
                Wordle::testout(
                    &format!("FAILED {}\n", &self.key_word.to_uppercase()).to_string(),
                    self.tty,
                );
                cnt = 0;
                break;
            }
        }
        (win_tag, cnt as u32, game)
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
struct Config {
    random: Option<bool>,
    difficult: Option<bool>,
    stats: Option<bool>,
    day: Option<u32>,
    seed: Option<u64>,
    final_set: Option<String>,
    acceptable_set: Option<String>,
    state: Option<String>,
    word: Option<String>,
}

impl Config {
    fn new() -> Config {
        Config {
            random: None,
            difficult: None,
            stats: None,
            day: None,
            seed: None,
            final_set: None,
            acceptable_set: None,
            state: None,
            word: None,
        }
    }
}

struct CliApp {
    cli_args: ArgMatches,
    config: Config,
}

impl CliApp {
    fn is_present(&self, arg: &str) -> bool {
        // let conf = self.get_conf(arg);
        match arg {
            "rand_mod" => {
                self.cli_args.is_present(arg)
                    | (self.config.random.is_some() && self.config.random.unwrap())
            }
            "hard_mod" => {
                self.cli_args.is_present(arg)
                    | (self.config.difficult.is_some() && self.config.difficult.unwrap())
            }
            "stats" => {
                self.cli_args.is_present(arg)
                    | (self.config.stats.is_some() && self.config.stats.unwrap())
            }
            "day" => self.cli_args.is_present(arg) | (self.config.day != None),
            "seed" => self.cli_args.is_present(arg) | (self.config.seed != None),
            "key_word" => self.cli_args.is_present(arg) | (self.config.word != None),
            "final_set_file" => self.cli_args.is_present(arg) | (self.config.final_set != None),
            "acceptable_set_file" => {
                self.cli_args.is_present(arg) | (self.config.acceptable_set != None)
            }
            "state_file" => self.cli_args.is_present(arg) | (self.config.state != None),
            _ => false,
        }
    }

    fn value_of(&self, arg: &str) -> Option<&str> {
        match arg {
            "key_word" => match &self.config.word {
                None => self.cli_args.value_of(arg),
                Some(s) => {
                    if self.cli_args.value_of(arg).is_some() {
                        self.cli_args.value_of(arg)
                    } else {
                        Some(s.as_str())
                    }
                }
            },
            "final_set_file" => match &self.config.final_set {
                None => self.cli_args.value_of(arg),
                Some(s) => {
                    if self.cli_args.value_of(arg).is_some() {
                        self.cli_args.value_of(arg)
                    } else {
                        Some(s.as_str())
                    }
                }
            },
            "acceptable_set_file" => match &self.config.acceptable_set {
                None => self.cli_args.value_of(arg),
                Some(s) => {
                    if self.cli_args.value_of(arg).is_some() {
                        self.cli_args.value_of(arg)
                    } else {
                        Some(s.as_str())
                    }
                }
            },
            "state_file" => match &self.config.state {
                None => self.cli_args.value_of(arg),
                Some(s) => {
                    if self.cli_args.value_of(arg).is_some() {
                        self.cli_args.value_of(arg)
                    } else {
                        Some(s.as_str())
                    }
                }
            },
            "day" => self.cli_args.value_of(arg),
            "seed" => self.cli_args.value_of(arg),
            _ => Some(""),
        }
    }

    fn new() -> CliApp {
        CliApp {
            cli_args: App::new("Wordle")
                .version("0.1.0")
                .author("Jashng")
                .about("A simple wordle game in Rust.")
                .arg(
                    Arg::with_name("key_word")
                        .short('w')
                        .long("word")
                        .takes_value(true)
                        .help("The key word for specifying the answer."),
                )
                .arg(
                    Arg::with_name("rand_mod")
                        .short('r')
                        .long("random")
                        .takes_value(false)
                        .help("Toggle to turn on random key word mode."),
                )
                .arg(
                    Arg::with_name("hard_mod")
                        .short('D')
                        .long("difficult")
                        .takes_value(false)
                        .help("Toggle to turn on difficult mode."),
                )
                .arg(
                    Arg::with_name("stats")
                        .short('t')
                        .long("stats")
                        .takes_value(false)
                        .help("Toggle to output your stats of the game after every single round."),
                )
                .arg(
                    Arg::with_name("day")
                        .short('d')
                        .long("day")
                        .takes_value(true)
                        .help("The day that you wanna start your game."),
                )
                .arg(
                    Arg::with_name("seed")
                        .short('s')
                        .long("seed")
                        .takes_value(true)
                        .help("The random seed for generating a key word."),
                )
                .arg(
                    Arg::with_name("final_set_file")
                        .short('f')
                        .long("final-set")
                        .takes_value(true)
                        .help("The file of the final set of the key word."),
                )
                .arg(
                    Arg::with_name("acceptable_set_file")
                        .short('a')
                        .long("acceptable-set")
                        .takes_value(true)
                        .help("The file of the acceptable set of the key word."),
                )
                .arg(
                    Arg::with_name("state_file")
                        .short('S')
                        .long("state")
                        .takes_value(true)
                        .help("The game state file to load previous games."),
                )
                .arg(
                    Arg::with_name("config")
                        .short('c')
                        .long("config")
                        .takes_value(true)
                        .help("The config file of input args."),
                )
                .get_matches(),
            config: Config::new(),
        }
    }
}

fn lines_from_file(filename: impl AsRef<Path>) -> io::Result<Vec<String>> {
    BufReader::new(File::open(filename)?).lines().collect()
}

fn game_day(
    matches: CliApp,
    first_tag: bool,
    day: u32,
    mut rounds: u32,
    mut win_rounds: u32,
    mut try_times: u32,
    mut words: HashMap<String, u32>,
    mut state: State,
    mut state_file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut final_set: Vec<String> = builtin_words::FINAL.iter().map(|s| s.to_string()).collect();
    let mut acceptable_set: Vec<String> = builtin_words::ACCEPTABLE
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut key_word: String = String::new();
    let mut seed: u64 = Wordle::SEED;
    let mut hard_mod: bool = false;
    let mut stats: bool = false;
    let tty: bool = atty::is(atty::Stream::Stdout);

    // arg hard_mod --difficult
    if matches.is_present("hard_mod") {
        hard_mod = true;
        if first_tag {
            Wordle::println("Difficult mode: on", tty, Some(true), Some(Color::Red));
        }
    }

    // arg stats --stats
    if matches.is_present("stats") {
        stats = true;
        if first_tag {
            Wordle::println(
                "Stats recording mode: on",
                tty,
                Some(true),
                Some(Color::Red),
            );
        }
    }

    // arg acceptable_set_file --acceptable-set
    if matches.is_present("acceptable_set_file") {
        match matches.value_of("acceptable_set_file") {
            None => return Err(ArgsErr("No input file of acceptable set found."))?,
            Some(pwd) => match pwd.parse::<String>() {
                Ok(path) => match lines_from_file(path) {
                    Ok(lines) => acceptable_set = lines,
                    Err(_) => return Err(ArgsErr("Could not load acceptable set."))?,
                },
                Err(_) => return Err(ArgsErr("File path has a wrong format."))?,
            },
        };
        acceptable_set = acceptable_set.iter().map(|s| s.to_lowercase()).collect();
        acceptable_set.sort_unstable();
        acceptable_set.dedup();

        for word in &acceptable_set {
            if word.len() != 5 {
                return Err(ArgsErr("The acceptable words set has incorrect word."))?;
            }
        }
    }

    // arg final_set_file --final-set
    if matches.is_present("final_set_file") {
        match matches.value_of("final_set_file") {
            None => return Err(ArgsErr("No input file of final set found."))?,
            Some(pwd) => match pwd.parse::<String>() {
                Ok(path) => match lines_from_file(path) {
                    Ok(lines) => final_set = lines,
                    Err(_) => return Err(ArgsErr("Could not load final set."))?,
                },
                Err(_) => return Err(ArgsErr("File path has a wrong format."))?,
            },
        };
        final_set = final_set.iter().map(|s| s.to_lowercase()).collect();
        final_set.sort_unstable();
        final_set.dedup();

        for word in &final_set {
            if word.len() != 5 {
                return Err(ArgsErr("The final words set has incorrect word."))?;
            }
        }
        let acc_set: HashSet<_> = acceptable_set.iter().cloned().collect();
        if !final_set.iter().all(|word| acc_set.contains(word)) {
            return Err(ArgsErr(
                "Every word in the final set should be covered in the acceptable set.",
            ))?;
        }
    }

    // handle args confict
    if (matches.is_present("seed") || matches.is_present("day")) && !matches.is_present("rand_mod")
    {
        return Err(ArgsErr(
            "-s/--seed and -d/--day can only be used in random mode.",
        ))?;
    }

    // arg: rand_mod --random
    if matches.is_present("rand_mod") {
        if matches.is_present("key_word") {
            return Err(ArgsErr("Random mode and key word input mode are conflict."))?;
        }
        if first_tag {
            Wordle::println("Random key word mode", tty, Some(true), Some(Color::Red));
        }
        let input_seed = matches.value_of("seed");
        match input_seed {
            None => {
                if matches.config.seed.is_some() {
                    seed = matches.config.seed.unwrap();
                }
            }
            Some(s) => match s.parse::<u64>() {
                Ok(se) => seed = se,
                Err(_) => return Err(ArgsErr("Your random seed must be a number of type <u64>."))?,
            },
        }
        let mut rng = StdRng::seed_from_u64(seed);
        final_set.shuffle(&mut rng);
        key_word = final_set[day as usize].to_string();
        Wordle::print("Random key: ", tty, Some(true), Some(Color::Blue));
        Wordle::println(&key_word, tty, Some(true), Some(Color::Green));
    } else {
        if matches.is_present("key_word") {
            let input_key_word = matches.value_of("key_word");
            match input_key_word {
                None => return Err( ArgsErr("No key word found after -w/--word."))?,
                Some(w) => {
                    match w.parse::<String>() {
                        Ok(wd) if wd.len() == 5 && final_set.contains(&wd) => {
                            if first_tag {
                                Wordle::print("Input key word found: ", tty, Some(true), Some(Color::Blue));
                                Wordle::println(&wd, tty, Some(true), Some(Color::Green));
                            }
                            key_word = wd;
                        },
                        _ => return Err( ArgsErr("The input key word has an incorrect format or not be in the final words set."))?,
                    }
                }
            };
        } else {
            loop {
                Wordle::print(
                    "Please input your key word: ",
                    tty,
                    Some(true),
                    Some(Color::Blue),
                );
                key_word = Wordle::read();
                if key_word.len() == 5 && final_set.contains(&key_word) {
                    break;
                } else {
                    Wordle::println("The input key word has an incorrect format or not be in the final words set.", tty, Some(true), Some(Color::Red));
                }
            }
        }
    }

    let mut wordle = Wordle::new(
        key_word,
        hard_mod,
        stats,
        seed,
        tty,
        final_set,
        acceptable_set,
    );

    let (win, try_time, new_game) = wordle.play(&mut words);
    rounds += 1;
    win_rounds += win;
    try_times += try_time;
    state.total_rounds += 1;
    state.games.push(new_game);
    if state_file_path != "".to_string() {
        let mut state_file = File::create(state_file_path)?;
        state_file.write_all(serde_json::to_string_pretty(&state)?.as_bytes())?;
    }

    // print stats
    if stats {
        // user output
        Wordle::println("\nYour Stats:", tty, Some(true), Some(Color::Green));
        Wordle::println(
            &format!(
                "Success rate: {}\nAverage trying times: {}",
                (win_rounds as f32) / (rounds as f32),
                match win_rounds {
                    0 => 0.0,
                    _ => (try_times as f32) / (win_rounds as f32),
                }
            )
            .to_string(),
            tty,
            None,
            None,
        );

        // test output
        Wordle::testout(
            &format!(
                "{} {} {:.2}\n",
                win_rounds,
                rounds - win_rounds,
                match win_rounds {
                    0 => 0.00,
                    _ => (try_times as f32) / (win_rounds as f32),
                }
            ),
            tty,
        );

        Wordle::println("Frequently used words:", tty, Some(true), Some(Color::Blue));
        let mut count_vec: Vec<(&String, &u32)> = words.iter().collect();
        count_vec.sort_by(|a, b| a.0.cmp(b.0));
        count_vec.sort_by(|a, b| b.1.cmp(a.1));
        for (index, value) in count_vec.iter().enumerate() {
            if index > 4 {
                break;
            }
            // user output
            Wordle::print(
                &format!("{}: {}; ", value.0, value.1).to_string(),
                tty,
                None,
                None,
            );
            // test output
            Wordle::testout(
                &format!(
                    "{}{} {}",
                    match &index {
                        0 => "",
                        _ => " ",
                    },
                    value.0.to_uppercase(),
                    value.1
                ),
                tty,
            );
        }
        Wordle::println("", tty, None, None);
        Wordle::testout("\n", tty);
    }

    Wordle::print(
        "Wanna play another round?(Y/N): ",
        tty,
        Some(true),
        Some(Color::Blue),
    );
    let choose: String = Wordle::read();
    if choose == "Y".to_string() {
        game_day(
            matches,
            false,
            day + 1,
            rounds,
            win_rounds,
            try_times,
            words,
            state,
            &state_file_path,
        )
    } else {
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // get the matches of args from command line
    let mut matches = CliApp::new();
    match matches.cli_args.value_of("config") {
        None => {}
        Some(pwd) => match pwd.parse::<String>() {
            Ok(path) => match File::open(&path) {
                Ok(file) => {
                    match serde_json::from_reader::<BufReader<std::fs::File>, Config>(
                        BufReader::new(file),
                    ) {
                        Ok(conf) => {
                            matches.config = conf;
                        }
                        Err(s) => return Err(s)?,
                    };
                }
                Err(_) => return Err(ArgsErr("No input file of args config found."))?,
            },
            Err(_) => return Err(ArgsErr("File path has a wrong format."))?,
        },
    };

    let mut day: u32 = 1;
    match matches.value_of("day") {
        None => {
            if matches.config.day.is_some() {
                day = matches.config.day.unwrap();
            }
        }
        Some(d) => match d.parse::<u32>() {
            Ok(dy) => {
                if dy < 1 {
                    return Err(ArgsErr("The arg 'day' must be a positive integer."))?;
                } else {
                    day = dy;
                }
            }
            Err(_) => return Err(ArgsErr("The format of -d/--day is wrong."))?,
        },
    };

    let mut state: State = State::new();
    let mut state_file = "".to_string();
    match matches.value_of("state_file") {
        None => {}
        Some(pwd) => match pwd.parse::<String>() {
            Ok(path) => match File::open(&path) {
                Ok(file) => {
                    state_file = path;
                    match serde_json::from_reader(BufReader::new(file)) {
                        Ok(st) => {
                            state = st;
                        }
                        Err(s) => return Err(s)?,
                    };
                }
                Err(_) => return Err(ArgsErr("No input file of previous game state found."))?,
            },
            Err(_) => return Err(ArgsErr("File path has a wrong format."))?,
        },
    };
    if state.games.len() != (state.total_rounds as usize) {
        return Err(ArgsErr("Total_rounds and game rounds doesn't match."))?;
    }
    let mut map: HashMap<String, u32> = HashMap::new();
    let mut win_rounds: u32 = 0;
    let mut try_times: u32 = 0;
    for game in &state.games {
        match game.guesses.len() {
            0 => {}
            len => {
                if game.answer == game.guesses[len - 1] {
                    win_rounds += 1;
                    try_times += len as u32;
                    for word in &game.guesses {
                        *map.entry(word.to_lowercase()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    game_day(
        matches,
        true,
        day - 1,
        state.total_rounds,
        win_rounds,
        try_times,
        map,
        state,
        &state_file,
    )
}
