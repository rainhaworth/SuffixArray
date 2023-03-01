use std::env;

use std::path::Path;
use std::fs::File;
use std::io::{self, BufRead, Read, Write};
use std::cmp;

use std::collections::HashMap;

use bisection::bisect_left_slice_by;

use std::time::Instant;

// from Rust docs:
// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

// define type alias so this is less awful to look at
type Inbytes = (String, Vec<usize>, u32, HashMap<String, (usize,usize)>);

// generate the smallest string that is lexicographically larger
// using alphabet ACGT
fn nextseq(queryseq: &String) -> String {
    let mut outstringvec = queryseq.chars().collect::<Vec<char>>();

    // define bidirectional map
    let idxtochar: HashMap<usize,char> = HashMap::from([
        (0, 'A'),
        (1, 'C'),
        (2, 'G'),
        (3, 'T')
    ]);
    let chartoidx: HashMap<char,usize> = HashMap::from([
        ('A', 0),
        ('C', 1),
        ('G', 2),
        ('T', 3)
    ]);

    // get string
    for i in 1..queryseq.len() {
        //iterate backwards
        let idx = queryseq.len() - i;
        let cti = chartoidx.get(&outstringvec[idx]).unwrap();
        // if this char isn't T, increment and exit loop
        if *cti < 3 {
            outstringvec[idx] = *idxtochar.get(&(*cti+1)).unwrap();
            break;
        }
        // if it's T, set to A and move to next char
        else {
            outstringvec[idx] = 'A';
            // if we're at the last char, push an A; doesn't matter where b/c the whole string is As
            if idx == 0 {
                outstringvec.push('A');
            }
        }
    }

    // return output string
    return outstringvec.into_iter().collect::<String>();
}

// actual query handler function
// not sure if this borrowing shit is gonna ruin my life but i'll find out i guess
fn runquery(decoded: &Inbytes, mode: bool, queryname: String, queryseq: &String) -> String {
    let mut outstr = String::new();
    let mut hitrange = (0usize, 0usize);
    let mut slice = (0usize, decoded.1.len());
    let k_usz = usize::try_from(decoded.2).unwrap();

    let now = Instant::now();

    // naive mode
    if mode == false {
        // if using prefix table, get slice to search
        if k_usz > 0 {
            let prefix = queryseq.chars().take(k_usz).collect::<String>();
            slice = decoded.3.get(&prefix).unwrap().clone();
        }
        // bisect_left to find start of range
        let start = bisect_left_slice_by(&decoded.1, slice.0..slice.1,
            |a| (&decoded.0)[*a..cmp::min(*a+queryseq.len(), decoded.0.len())].cmp(&queryseq));
        
        // get next sequence in lexicographical order and use to find end of range
        let ns = nextseq(queryseq);
        let end = bisect_left_slice_by(&decoded.1, slice.0..slice.1,
            |a| (&decoded.0)[*a..cmp::min(*a+ns.len(), decoded.0.len())].cmp(&ns));

        // save hit range
        hitrange = (start, end);
    }
    
    // add query name, number of hits to outstr
    outstr.push_str(format!("{}\t{}", queryname, hitrange.1 - hitrange.0).as_str());

    // add list of hits
    for hit in hitrange.0..hitrange.1 {
        // extract index from suffix array
        outstr.push_str(format!("\t{}", decoded.1[hit]).as_str());
    }

    // add newline
    outstr.push('\n');

    let elapsed_time = now.elapsed();
    println!("{} query runtime: {:?}", &queryname, elapsed_time);

    // return
    return outstr;
}

// given suffix array path, query sequence path, mode, and output name, query suffix array
fn querysa(index: &Path, queries: &Path, mode: bool, output: String){
    // mode: 0 = naive, 1 = simpaccel

    // load suffix array and prefix table from file
    // check if prefix table is empty
    // return file contents (binary)
    fn read_index_file(filepath: &Path) -> io::Result<Vec<u8>> {
        let mut f = File::open(filepath)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        Ok(buffer)
    }

    // read in bytes and decode with bincode
    let bytes = read_index_file(index).unwrap();
    let decoded: Inbytes = bincode::deserialize(&bytes).unwrap();

    // decoded.0 --> reference sequence
    // decoded.1 --> suffix array
    // decoded.2 --> k
    // decoded.3 --> prefix table
    
    // load queries file using read_lines iterator
    let mut outstr = String::new();
    let mut queryname = String::new();
    let mut queryseq = String::new();
    if let Ok(lines) = read_lines(queries) {
        // Consumes the iterator, returns an (Optional) String
        for line in lines {
            if let Ok(ip) = line {
                // get name from header
                if ip.chars().nth(0).unwrap() == '>' {
                    // if we have a query sequence, handle it
                    if !queryseq.is_empty() {
                        outstr.push_str(&runquery(&decoded, mode, queryname, &queryseq));
                        queryseq.clear();
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
        // handle last query sequence
        if !queryseq.is_empty() {
            outstr.push_str(&runquery(&decoded, mode, queryname, &queryseq));
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
