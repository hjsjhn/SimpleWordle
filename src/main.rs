mod builtin_words;

use console;
use serde_json;
use std::{
    fs::File,
    io::{self, Write, BufRead, BufReader},
    path::Path,
    collections::{HashMap, HashSet},
};
use clap::{Arg, App, ArgMatches};
use rand::{SeedableRng, rngs::StdRng, seq::SliceRandom};
use serde_derive::{Serialize, Deserialize};


#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
struct Game {
    answer: String,
    guesses: Vec<String>,
}

impl Game {
    fn new() -> Game {
        Game { answer: "".to_string(), guesses: vec![] }
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
        State { total_rounds: 0, games: vec![] }
    }
}


#[derive(Debug)]
struct ArgsErr<'a> (&'a str);
impl std::fmt::Display for ArgsErr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "ArgsError: {}", self.0)
    }
}
impl std::error::Error for ArgsErr<'_> {}


#[derive(Copy)]
#[derive(Clone)]
#[derive(Debug)]
#[derive(PartialEq)]
enum AlphStatus{ //the status of alphabet
    Right,
    PosWrong,
    TooMany,
    Unknown,
}

impl AlphStatus {
    fn parse1(&self) -> u32 { //parse for comparing priority
        match &self {
            AlphStatus::Right => 3,
            AlphStatus::PosWrong => 2,
            AlphStatus::TooMany => 1,
            AlphStatus::Unknown => 0,
        }
    }

    fn parse2(&self) -> u32 { //parse for getting color code (None,R,G,B,Y)->(0,1,2,3,4)
        match &self {
            AlphStatus::Right => 2, //Green
            AlphStatus::PosWrong => 4, //Yellow
            AlphStatus::TooMany => 1, //Red
            AlphStatus::Unknown => 0, //None
        }
    }

    fn parse3(&self) -> String { //parse for getting status G,Y,R,X
        match &self {
            AlphStatus::Right => "G".to_string(), //Green
            AlphStatus::PosWrong => "Y".to_string(), //Yellow
            AlphStatus::TooMany => "R".to_string(), //Red
            AlphStatus::Unknown => "X".to_string(), //Unknown
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

    fn printall(pln: bool, words: &str, tty: bool, bold: Option<bool>, color: Option<u32>) {
        if tty {
            // color: 0->nothing 1->red 2->green 3->blue 4->yellow
            let bd: bool = bold.unwrap_or(false);
            let mut col: u32 = color.unwrap_or(0);
            if col > 4 { col = 0; }
            let mut stl = console::style(words.to_string());
            if bd { stl = stl.bold(); }
            match col {
                1 => stl = stl.red(),
                2 => stl = stl.green(),
                3 => stl = stl.blue(),
                4 => stl = stl.yellow(),
                _ => {},
            };
            match pln {
                true => println!("{}", stl),
                false => print!("{}", stl),
            };
            io::stdout().flush().unwrap();
        }
    }

    fn println(words: &str, tty: bool, bold: Option<bool>, color: Option<u32>) {
        Wordle::printall(true, words, tty, bold, color);
    }

    fn print(words: &str, tty: bool, bold: Option<bool>, color: Option<u32>) {
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

    fn new(key_word: String,
        hard_mod: bool,
        stats: bool,
        seed: u64,
        tty: bool,
        final_set: Vec<String>,
        acceptable_set: Vec<String>) -> Wordle {
        Wordle { key_word: key_word, hard_mod: hard_mod, stats: stats, seed: seed, tty: tty, final_set: final_set, acceptable_set: acceptable_set }
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

    fn check_hard_mod(&self, input_word: &str, curstatus: &Vec<AlphStatus>, status: &HashMap<char, AlphStatus>) -> bool {
        if !self.hard_mod { return true; }
        let input: String = input_word.to_string();
        let mut tmp:usize = 0;
        let mut ninput: Vec<char> = vec![];
        for (&c1, &c2) in self.key_word.chars().collect::<Vec<char>>().iter().zip(input.chars().collect::<Vec<char>>().iter()) {
            // println!("{}: {:?}", tmp, curstatus[tmp]);
            if curstatus[tmp] == AlphStatus::Right {
                if c1 != c2 { return false; }
            } else {
                ninput.push(c2);
            }
            tmp += 1;
        }
        for c in Wordle::ALPHABET.chars() {
            if *status.get(&c).unwrap() == AlphStatus::PosWrong {
                if !ninput.contains(&c) { return false; }
            }
        }
        true
    }

    fn play(&self, words_map: &mut HashMap<String, u32>) -> (u32, u32, Game) {
        let mut cnt: usize = 0;
        let mut win_tag: u32 = 0;
        let mut status = HashMap::new();
        for c in Wordle::ALPHABET.chars() {
            status.insert(c, AlphStatus::Unknown);
        }
        let mut curstatus: Vec<AlphStatus> = vec![AlphStatus::TooMany; 5];
        let mut game = Game::new();
        game.answer = self.key_word.to_string().to_uppercase();
        loop {
            cnt = cnt + 1;
            let mut input_word = String::new();
            Wordle::print(&format!("Start Guessing({}): ", Wordle::trans_to_onum(cnt)).to_string(), self.tty, Some(true), Some(3));
            // println!("{:?}", curstatus);
            loop {
                input_word = Wordle::read();
                if input_word.len() == 5 && self.acceptable_set.contains(&input_word) && self.check_hard_mod(&input_word, &curstatus, &status) {
                    break;
                } else {
                    Wordle::print("Key word format error or not in word list. Input again: ", self.tty, Some(false), Some(1));
                    Wordle::testout("INVALID\n", self.tty);
                }
            }
        
            game.guesses.push(input_word.to_string().to_uppercase());
            *words_map.entry(input_word.to_string()).or_insert(0) += 1;

            //update status of the word
            let mut map = HashMap::new();
            curstatus = vec![AlphStatus::TooMany; 5];
            let mut tmp:usize = 0;
            for (&c1, &c2) in self.key_word.chars().collect::<Vec<char>>().iter().zip(input_word.chars().collect::<Vec<char>>().iter()) {
                let count = map.entry(c1).or_insert(0);
                if c1 == c2 {
                    curstatus[tmp] = AlphStatus::Right;
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
                    *count -= 1;
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
                Wordle::print(&c.to_string(), self.tty, Some(false), Some(curstatus[tmp].parse2()));
                tmp += 1;
            }
            Wordle::println("", self.tty, None, None);
            for c in Wordle::ALPHABET.chars() {
                Wordle::print(&c.to_string(), self.tty, Some(false), Some(status.get(&c).unwrap().parse2()));
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
                Wordle::println(&format!("CORRECT, guess time: {}", cnt).to_string(), self.tty, Some(true), Some(2));
                Wordle::testout(&format!("CORRECT {}\n", cnt).to_string(), self.tty);
                win_tag = 1;
                break;
            }
            if cnt == 6 {
                Wordle::print("LOST, you failed too many times.", self.tty, Some(true), Some(1));
                Wordle::testout(&format!("FAILED {}\n", &self.key_word.to_uppercase()).to_string(), self.tty);
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
        Config { random: None, difficult: None, stats: None, day: None, seed: None, final_set: None, acceptable_set: None, state: None, word: None }
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
            "rand_mod" => self.cli_args.is_present(arg) | (self.config.random.is_some() && self.config.random.unwrap()),
            "hard_mod" => self.cli_args.is_present(arg) | (self.config.difficult.is_some() && self.config.difficult.unwrap()),
            "stats" => self.cli_args.is_present(arg) | (self.config.stats.is_some() && self.config.stats.unwrap()),
            "day" => self.cli_args.is_present(arg) | (self.config.day != None),
            "seed" => self.cli_args.is_present(arg) | (self.config.seed != None),
            "key_word" => self.cli_args.is_present(arg) | (self.config.word != None),
            "final_set_file" => self.cli_args.is_present(arg) | (self.config.final_set != None),
            "acceptable_set_file" => self.cli_args.is_present(arg) | (self.config.acceptable_set != None),
            "state_file" => self.cli_args.is_present(arg) | (self.config.state != None),
            _ => false,
        }
    }
    
    fn value_of(&self, arg: &str) -> Option<&str> {
        match arg {
            "key_word" => { match &self.config.word { None => self.cli_args.value_of(arg), Some(s) => { if self.cli_args.value_of(arg).is_some() { self.cli_args.value_of(arg) } else { Some(s.as_str()) }} , }},
            "final_set_file" => { match &self.config.final_set { None => self.cli_args.value_of(arg), Some(s) => { if self.cli_args.value_of(arg).is_some() { self.cli_args.value_of(arg) } else { Some(s.as_str()) }} , }},
            "acceptable_set_file" => { match &self.config.acceptable_set { None => self.cli_args.value_of(arg), Some(s) => { if self.cli_args.value_of(arg).is_some() { self.cli_args.value_of(arg) } else { Some(s.as_str()) }} , }},
            "state_file" => { match &self.config.state { None => self.cli_args.value_of(arg), Some(s) => { if self.cli_args.value_of(arg).is_some() { self.cli_args.value_of(arg) } else { Some(s.as_str()) }} , }},
            "day" => self.cli_args.value_of(arg),
            "seed" => self.cli_args.value_of(arg),
            _ => Some(""),
        }
    }

    fn new() -> CliApp {
        CliApp { cli_args:
             App::new("Wordle")
            .version("0.1.0")
            .author("Jashng")
            .about("A simple wordle game in Rust.")
            .arg(Arg::with_name("key_word")
                    .short('w')
                    .long("word")
                    .takes_value(true)
                    .help("The key word for specifying the answer."))
            .arg(Arg::with_name("rand_mod")
                    .short('r')
                    .long("random")
                    .takes_value(false)
                    .help("Toggle to turn on random key word mode."))
            .arg(Arg::with_name("hard_mod")
                    .short('D')
                    .long("difficult")
                    .takes_value(false)
                    .help("Toggle to turn on difficult mode."))
            .arg(Arg::with_name("stats")
                    .short('t')
                    .long("stats")
                    .takes_value(false)
                    .help("Toggle to output your stats of the game after every single round.")) 
            .arg(Arg::with_name("day")
                    .short('d')
                    .long("day")
                    .takes_value(true)
                    .help("The day that you wanna start your game."))
            .arg(Arg::with_name("seed")
                    .short('s')
                    .long("seed")
                    .takes_value(true)
                    .help("The random seed for generating a key word."))
            .arg(Arg::with_name("final_set_file")
                    .short('f')
                    .long("final-set")
                    .takes_value(true)
                    .help("The file of the final set of the key word."))
            .arg(Arg::with_name("acceptable_set_file")
                    .short('a')
                    .long("acceptable-set")
                    .takes_value(true)
                    .help("The file of the acceptable set of the key word."))
            .arg(Arg::with_name("state_file")
                    .short('S')
                    .long("state")
                    .takes_value(true)
                    .help("The game state file to load previous games."))
            .arg(Arg::with_name("config")
                    .short('c')
                    .long("config")
                    .takes_value(true)
                    .help("The config file of input args."))
            .get_matches(), 
            config: Config::new(), }
    }
}


fn lines_from_file(filename: impl AsRef<Path>) -> io::Result<Vec<String>> {
    BufReader::new(File::open(filename)?).lines().collect()
}


fn game_day(matches: CliApp, first_tag: bool, day: u32, mut rounds: u32, mut win_rounds: u32, mut try_times: u32, mut words: HashMap<String, u32>, mut state: State, mut state_file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut final_set: Vec<String> = builtin_words::FINAL.iter().map(|s| s.to_string()).collect();
    let mut acceptable_set: Vec<String> = builtin_words::ACCEPTABLE.iter().map(|s| s.to_string()).collect();
    let mut key_word: String = String::new();
    let mut seed: u64 = Wordle::SEED;
    let mut hard_mod: bool = false;
    let mut stats: bool = false;
    let tty: bool = atty::is(atty::Stream::Stdout);

    // arg hard_mod --difficult
    if matches.is_present("hard_mod") {
        hard_mod = true;
        if first_tag { Wordle::println("Difficult mode: on", tty, Some(true), Some(1)); }
    }

    // arg stats --stats
    if matches.is_present("stats") {
        stats = true;
        if first_tag { Wordle::println("Stats recording mode: on", tty, Some(true), Some(1)); }
    }

    // arg acceptable_set_file --acceptable-set
    if matches.is_present("acceptable_set_file") {
        match matches.value_of("acceptable_set_file") {
            None => return Err (ArgsErr("No input file of acceptable set found."))?,
            Some(pwd) => {
                match pwd.parse::<String>() {
                    Ok(path) => {
                        match lines_from_file(path) {
                            Ok(lines) => acceptable_set = lines,
                            Err(_) => return Err (ArgsErr("Could not load acceptable set."))?,
                        }
                    }
                    Err(_) => return Err (ArgsErr("File path has a wrong format."))?,
                }
            }
        };
        acceptable_set = acceptable_set.iter().map(|s| s.to_lowercase()).collect();
        acceptable_set.sort_unstable();
        acceptable_set.dedup();

        for word in &acceptable_set {
            if word.len() != 5 { return Err (ArgsErr("The acceptable words set has incorrect word."))?; }
        }
    }

    // arg final_set_file --final-set
    if matches.is_present("final_set_file") {
        match matches.value_of("final_set_file") {
            None => return Err (ArgsErr("No input file of final set found."))?,
            Some(pwd) => {
                match pwd.parse::<String>() {
                    Ok(path) => {
                        match lines_from_file(path) {
                            Ok(lines) => final_set = lines,
                            Err(_) => return Err (ArgsErr("Could not load final set."))?,
                        }
                    }
                    Err(_) => return Err (ArgsErr("File path has a wrong format."))?,
                }
            }
        };
        final_set = final_set.iter().map(|s| s.to_lowercase()).collect();
        final_set.sort_unstable();
        final_set.dedup();

        for word in &final_set {
            if word.len() != 5 { return Err (ArgsErr("The final words set has incorrect word."))?; }
        }
        let acc_set: HashSet<_> = acceptable_set.iter().cloned().collect();
        if !final_set.iter().all(|word| acc_set.contains(word)) { return Err (ArgsErr("Every word in the final set should be covered in the acceptable set."))?; }
    }

    // handle args confict
    if (matches.is_present("seed") || matches.is_present("day")) && !matches.is_present("rand_mod") {
        return Err (ArgsErr("-s/--seed and -d/--day can only be used in random mode."))?;
    }

    // arg: rand_mod --random
    if matches.is_present("rand_mod") {
        if matches.is_present("key_word") { return Err( ArgsErr("Random mode and key word input mode are conflict."))?; }
        if first_tag { Wordle::println("Random key word mode", tty, Some(true), Some(1)); }
        let input_seed = matches.value_of("seed");
        match input_seed {
            None => {
                if matches.config.seed.is_some() {
                    seed = matches.config.seed.unwrap();
                }
            }
            Some(s) => {
                match s.parse::<u64>() {
                    Ok(se) => seed = se,
                    Err(_) => return Err( ArgsErr("Your random seed must be a number of type <u64>."))?,
                }
            }
        }
        let mut rng = StdRng::seed_from_u64(seed);
        final_set.shuffle(&mut rng);
        key_word = final_set[day as usize].to_string();
        Wordle::print("Random key: ", tty, Some(true), Some(3));
        Wordle::println(&key_word, tty, Some(true), Some(2));
    } else {
        if matches.is_present("key_word") {
            let input_key_word = matches.value_of("key_word");
            match input_key_word {
                None => return Err( ArgsErr("No key word found after -w/--word."))?,
                Some(w) => {
                    match w.parse::<String>() {
                        Ok(wd) if wd.len() == 5 && final_set.contains(&wd) => {
                            if first_tag {
                                Wordle::print("Input key word found: ", tty, Some(true), Some(3));
                                Wordle::println(&wd, tty, Some(true), Some(2));
                            }
                            key_word = wd;
                        },
                        _ => return Err( ArgsErr("The input key word has an incorrect format or not be in the final words set."))?,
                    }
                }
            };
        } else {
            loop {
                Wordle::print("Please input your key word: ", tty, Some(true), Some(3));
                key_word = Wordle::read();
                if key_word.len() == 5 && final_set.contains(&key_word) { break; }
                else { Wordle::println("The input key word has an incorrect format or not be in the final words set.", tty, Some(true), Some(1)); }
            }
        }
    }
    let mut wordle = Wordle::new(key_word, hard_mod, stats, seed, tty, final_set, acceptable_set);

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
        Wordle::println("\nYour Stats:", tty, Some(true), Some(2));
        Wordle::println(&format!("Success rate: {}\nAverage trying times: {}", (win_rounds as f32) / (rounds as f32), match win_rounds { 0 => 0.0, _ => (try_times as f32) / (win_rounds as f32), }).to_string(), tty, None, None);

        // test output
        Wordle::testout(&format!("{} {} {:.2}\n", win_rounds, rounds - win_rounds, match win_rounds { 0 => 0.00, _ => (try_times as f32) / (win_rounds as f32), }), tty);

        Wordle::println("Frequently used words:", tty, Some(true), Some(3));
        let mut count_vec: Vec<(&String, &u32)> = words.iter().collect();
        count_vec.sort_by(|a, b| a.0.cmp(b.0));
        count_vec.sort_by(|a, b| b.1.cmp(a.1));
        for (index, value) in count_vec.iter().enumerate() {
            if index > 4 { break; }
            // user output
            Wordle::print(&format!("{}: {}; ", value.0, value.1).to_string(), tty, None, None);
            // test output
            Wordle::testout(&format!("{}{} {}", match &index { 0 => "", _ => " ", }, value.0.to_uppercase(), value.1), tty);
        }
        Wordle::println("", tty, None, None);
        Wordle::testout("\n", tty);
    }

    Wordle::print("Wanna play another round?(Y/N): ", tty, Some(true), Some(3));
    let choose: String = Wordle::read();
    if choose == "Y".to_string() { game_day(matches, false, day + 1, rounds, win_rounds, try_times, words, state, &state_file_path) }
    else { Ok(()) }
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    // get the matches of args from command line
    let mut matches = CliApp::new();
    match matches.cli_args.value_of("config") {
        None => {},
        Some(pwd) => {
            match pwd.parse::<String>() {
                Ok(path) => {
                    match File::open(&path) {
                        Ok(file) => {
                            match serde_json::from_reader::<BufReader<std::fs::File>, Config>(BufReader::new(file)) {
                                Ok(conf) => {
                                    matches.config = conf;
                                },
                                Err(s) => return Err (s)?,
                            };
                        },
                        Err(_) => return Err (ArgsErr("No input file of args config found."))?,
                    }
                }
                Err(_) => return Err (ArgsErr("File path has a wrong format."))?,
            }
        }
    };

    let mut day: u32 = 1;
    match matches.value_of("day") {
        None => {
            if matches.config.day.is_some() {
                day = matches.config.day.unwrap();
            }
        },
        Some(d) => {
            match d.parse::<u32>() {
                Ok(dy) => day = dy,
                Err(_) => return Err( ArgsErr("The format of -d/--day is wrong."))?,
            }
        }
    };

    let mut state: State = State::new();
    let mut state_file = "".to_string();
    match matches.value_of("state_file") {
        None => {},
        Some(pwd) => {
            match pwd.parse::<String>() {
                Ok(path) => {
                    match File::open(&path) {
                        Ok(file) => {
                            state_file = path;
                            match serde_json::from_reader(BufReader::new(file)) {
                                Ok(st) => {
                                    state = st;
                                },
                                Err(s) => return Err (s)?,
                            };
                        },
                        Err(_) => return Err (ArgsErr("No input file of previous game state found."))?,
                    }
                }
                Err(_) => return Err (ArgsErr("File path has a wrong format."))?,
            }
        }
    };
    if state.games.len() != (state.total_rounds as usize) { return Err (ArgsErr("Total_rounds and game rounds doesn't match."))?; }
    let mut map: HashMap<String, u32> = HashMap::new();
    let mut win_rounds: u32 = 0;
    let mut try_times: u32 = 0;
    for game in &state.games {
        match game.guesses.len() {
            0 => {},
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

    game_day(matches, true, day - 1, state.total_rounds, win_rounds, try_times, map, state, &state_file)
}
