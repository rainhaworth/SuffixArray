use std::env;

use std::path::Path;
use std::fs::File;
use std::io::{self, BufRead, Read, Write};

use std::collections::HashMap;

use rkyv;
use rkyv::Deserialize;

// from Rust docs:
// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

// define type alias so this is less awful to look at
type Outfile = (Vec<(usize, Vec<char>)>, HashMap<String, (usize,usize)>, u32);

// gonna just write this here for now or i have to stop using cargo i think
// given 
fn querysa(index: &Path, queries: &Path, mode: bool, output: String){
    // mode: 0 = naive, 1 = simpaccel

    // load suffix array and prefix table from file
    // check if prefix table is empty
    //return file contents (binary)
    fn read_index_file(filepath: &Path) -> io::Result<Vec<u8>> {
        let mut f = File::open(filepath)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        Ok(buffer)
    }

    let bytes = read_index_file(index).unwrap();

    // get archive and deserialize
    let deserealized: Outfile = rkyv::check_archived_root::<Outfile>(&bytes[..]).unwrap()
        .deserialize(&mut rkyv::Infallible).unwrap();

    // deserealized.0 --> suffix array
    // deserealized.1 --> prefix table
    let k_usz = usize::try_from(deserealized.2).unwrap();

    // load queries file using read_lines iterator
    let mut outstr = String::new();
    let mut queryname = String::new();
    let mut queryseq = String::new();
    let mut hits: Vec<usize> = Vec::new();
    let mut slice = (0usize, deserealized.0.len());
    if let Ok(lines) = read_lines(queries) {
        // Consumes the iterator, returns an (Optional) String
        for line in lines {
            if let Ok(ip) = line {
                // get name from header
                if ip.chars().nth(0).unwrap() == '>' {
                    // if we have a query string, handle it
                    if !queryseq.is_empty() {
                        // naive mode
                        if mode == false {
                            // if using prefix table, get slice to search
                            if k_usz > 0 {
                                let prefix = queryseq.chars().take(k_usz).collect::<String>();
                                slice = deserealized.1.get(&prefix).unwrap().clone();
                            }

                            // search entire suffix array
                            // this is a complete mess but basically,
                                // 1. get a slice of the suffix array
                                // 2. extract a chunk of the string to compare to the query sequence
                                // 3. binary search
                                // 4. if we find a hit, traverse the suffix array to find all the others
                            let search = deserealized.0[slice.0..slice.1].binary_search_by
                                (|(_a,b)| b.get(0..queryseq.len())
                                .unwrap().iter().collect::<String>().cmp(&queryseq));

                            match search {
                                Ok(i) => hits.push(i),
                                Err(_e) => ()
                            }

                            if !hits.is_empty() {
                                let hit_idx = hits.last().unwrap().clone();
                                let mut i = hit_idx;
                                let mut direction = true;
                                // look forward (true) then backward (false)
                                loop {
                                    if direction == true {
                                        i += 1;
                                    } else {
                                        i -= 1;
                                    }

                                    if deserealized.0[i].1.starts_with(&queryseq.chars().collect::<Vec<char>>()) {
                                        hits.push(i);
                                    } else if direction == true {
                                        direction = false;
                                        i = hit_idx;
                                    } else {
                                        break;
                                    }
                                }
                            }

                            // reset query sequence and search slice
                            slice = (0usize, deserealized.0.len());
                            queryseq.clear();
                        }
                        
                        // add query name, number of hits to outstr
                        outstr.push_str(format!("{}\t{}", queryname, hits.len()).as_str());

                        // add list of hits
                        for hit in &hits {
                            // extract index from suffix array
                            let idx = deserealized.0[*hit].0;
                            outstr.push_str(format!("\t{}", idx).as_str());
                        }

                        // add newline
                        outstr.push('\n');

                        // reset hits
                        hits.clear();
                    }

                    // update query name
                    queryname = ip[1..].to_string();

                }
                // if non header line, get query sequence
                else if queryseq.is_empty() {
                    queryseq = ip;
                }
                else {
                    queryseq.push_str(&ip); // this is probably suboptimal b/c of reallocation
                }
            }
        }
    }

    // write to output file
    let mut file = match File::create(&output) {
        Err(e) => panic!("couldn't create {}: {}", output, e),
        Ok(file) => file,
    };
    
    match write!(file, "{}", outstr) {
        Err(e) => panic!("couldn't write to {}: {}", output, e),
        Ok(_) => println!("successfully wrote to {}", output),
    }

}

fn main() {
    // fetch data from command line args
    // i should have used a crate for this but it's fine, it works
    let mut index_str = String::new();
    let mut queries_str = String::new();
    let mut mode_str = String::new();
    let mut output = String::new();
    let mut i: usize = 1; //skip first arg
    loop {
        if let Some(arg) = env::args().nth(i){
            if index_str.is_empty() {
                index_str = arg;
            }
            else if queries_str.is_empty() {
                queries_str = arg;
            }
            else if mode_str.is_empty() {
                mode_str = arg;
            }
            else if output.is_empty() {
                output = arg;
            } else {
                break;
            }
            i += 1;
        } else if output.is_empty(){
            panic!("Not enough arguments")
        } else {
            break;
        }
    }

    // validate mode
    if mode_str != "naive" && mode_str != "simpaccel" {
        panic!("Invalid mode");
    }

    // make path from string, run function
    let index = Path::new(&index_str);
    let queries = Path::new(&queries_str);
    let mode = mode_str == "simpaccel";
    querysa(index, queries, mode, output);
}
