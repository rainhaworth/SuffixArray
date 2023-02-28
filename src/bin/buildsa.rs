use std::env;

use std::path::Path;
use std::fs::File;
use std::io::{self, BufRead, Write};

use std::collections::HashMap;

use rkyv;

use std::time::Instant;

// from Rust docs:
// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

// given input data, build suffix array, encode, and write to file
fn buildsa(reference: &Path, output: String, k: u32) {
    // read in FASTA file, store as string (first entry, i.e. until '>'? until EOF?)
    let mut refseq = String::new();

    // from Rust docs:
    // File hosts must exist in current path before this produces output
    if let Ok(lines) = read_lines(reference) {
        // Consumes the iterator, returns an (Optional) String
        for line in lines {
            if let Ok(ip) = line {
                // skip header
                if ip.chars().nth(0).unwrap() == '>' {
                    continue;
                }
                else if refseq.is_empty() {
                    refseq = ip;
                }
                else {
                    refseq.push_str(&ip); // this is probably suboptimal b/c of reallocation
                }
            }
        }
    }

    //append $
    refseq.push('$');

    // create suffix array
    // vector where each element is a tuple of (index, suffix), where suffix is a char vector
    let reflen = refseq.len();
    let mut suffixvec: Vec<(usize, Vec<char>)> = vec![(0, Vec::with_capacity(reflen)); reflen];
    
    // populate
    let refseqvec: Vec<char> = refseq.chars().collect();
    for i in 0..reflen {
        suffixvec[i] = (i, refseqvec[i..reflen].to_vec());
    }

    /*for i in (reflen-5)..reflen {
        println!("{}: {}", suffixvec[i].0, suffixvec[i].1.iter().cloned().collect::<String>())
    }*/

    // sort lexicographically, i.e. only using suffixes
    suffixvec.sort_unstable_by(|(_a,b), (_c,d)| b.cmp(d));

    // prefix table
    let mut prefmap: HashMap<String, (usize,usize)> = HashMap::new();
    if k != 0 {
        // map all canonical k-mers to a range
        // e.g., k=3, ('ACG',(50,60)) --> range of [50,60)
        let k_usz = usize::try_from(k).unwrap();
        let mut start: usize = 0;
        let mut prefix = String::new();

        for i in 0..suffixvec.len() {
            // skip entries that are too small
            // this might cause weird behavior; if so, handle this and exclude things from the range
            if suffixvec[i].1.len() < k_usz{
                continue;
            }
            // grab prefix slice
            let prefix_new = suffixvec[i].1.get(0..k_usz).unwrap().iter().collect::<String>();

            // if different prefix is found, update prefix table
            if prefix_new != prefix {
                if !prefix.is_empty() {
                    prefmap.insert(prefix.clone(), (start, i));
                }
                prefix = prefix_new;
                start = i;
            }
        }

        // add last k-mer; i don't think there's a case where we don't need to do this
        prefmap.insert(prefix.clone(), (start, suffixvec.len()));
    }

    // generate binary encoding w/ rkyv
    // format: (Vec<(usize, Vec<char>)>, HashMap<String, (usize,usize)>, u32)
    // i tried making this into a struct but everything broke, so i'm doing this instead
    let bytes = rkyv::to_bytes::<_, 256>(&(suffixvec, prefmap, k)).unwrap();

    // write to file (from Rust docs)
    let mut file = match File::create(&output) {
        Err(e) => panic!("couldn't create {}: {}", output, e),
        Ok(file) => file,
    };
    
    match file.write_all(&bytes) {
        Err(e) => panic!("couldn't write to {}: {}", output, e),
        Ok(_) => println!("successfully wrote to {}", output),
    }

}

fn main() {
    // fetch data from command line args
    // i should have used a crate for this but it's fine, it works
    let mut k: u32 = 0;
    let mut reference_str = String::new();
    let mut output = String::new();
    let mut i: usize = 1; //skip first arg
    loop {
        if let Some(arg) = env::args().nth(i){
            if arg == "--preftab" {
                if let Some(tmp) = env::args().nth(i+1){
                    k = tmp.parse::<u32>().unwrap();
                    i += 2;
                } else {
                    panic!("No value supplied after --preftab");
                }
            } else if reference_str.is_empty() {
                reference_str = arg;
                i += 1;
            } else if output.is_empty() {
                output = arg;
                i += 1;
            } else {
                break;
            }
        } else if reference_str.is_empty() || output.is_empty(){
            panic!("Not enough arguments")
        } else {
            break;
        }
    }
    let now = Instant::now();
    // make path from string, run function
    let reference = Path::new(&reference_str);
    buildsa(reference, output, k);

    let elapsed = now.elapsed();
    println!("runtime: {} ms", elapsed.as_millis());
}
