use std::io::{stdin, stdout, Result};
use std::io::{Read, Write, Error, ErrorKind};
use std::mem::{size_of, MaybeUninit};

#[derive(Clone)]
enum TodoItem {
    Task(bool, String),
    Group(String, Vec<TodoItem>),
}
impl TodoItem {
    fn completed(&self) -> bool {
        match self {
            TodoItem::Task(c, _) => *c,
            TodoItem::Group(_, xs) => {
                let mut _bv = true;
                for x in xs {
                    _bv &= x.completed();
                }
                _bv
            }
        }
    }
    fn complete(&mut self, value: bool) {
        match self {
            TodoItem::Task(_, msg) => {
                *self = TodoItem::Task(value, msg.to_string());
            }
            TodoItem::Group(_, xs) => {
                for x in xs.iter_mut() {
                    x.complete(value)
                }
            }
        }
    }
    fn message(&self) -> &str {
        match self {
            TodoItem::Task(_, msg) => msg,
            TodoItem::Group(msg, _) => msg,
        }
    }
    fn render<W: Write>(&self, depth: u8, outp: &mut W, sel: Option<&Selection>) -> Result<()> {
        self.render_depth(depth, outp, sel.and_then(|sel| Some((sel, 0))))
    }
    fn render_depth<W: Write>(
        &self,
        depth: u8,
        outp: &mut W,
        sel: Option<(&Selection, usize)>,
    ) -> Result<()> {
        let selected = sel.is_some() && {
            let (s, i) = sel.unwrap();
            i == s.0.len()
        };
        let msg = self.message();
        let mut out = String::with_capacity(depth as usize + 5 + msg.len());
        if selected {
            out.extend("\x1b[7m".chars());
        }
        for _ in 0..depth {
            out.push('\t');
        }

        if self.completed() {
            out.extend("[#] ".chars());
        } else {
            out.extend("[ ] ".chars());
        }
        out.extend(msg.chars());
        out.push('\n');
        outp.write(out.as_bytes())?;
        if let TodoItem::Group(_, xs) = self {
            for (i, x) in xs.into_iter().enumerate() {
                let fsel = if selected {
                    None
                } else {
                    sel.and_then(|sel| {
                        if sel.0.0[sel.1] as usize == i {
                            Some((sel.0, sel.1 + 1))
                        } else {
                            None
                        }
                    })
                };
                x.render_depth(depth + 1, outp, fsel)?;
            }
        }
        if selected {
            outp.write("\x1b[0m".as_bytes())?;
        }
        Ok(())
    }
    fn get(&self, sel: &Selection) -> Option<&Self> {
        let mut cur = self;
        for i in &(sel.0) {
            match cur {
                TodoItem::Task(_, _) => {
                    return None;
                }
                TodoItem::Group(_, xs) => match xs.get(*i as usize) {
                    Some(x) => cur = x,
                    None => {
                        return None;
                    }
                },
            }
        }
        Some(cur)
    }
    fn get_mut(&mut self, sel: &Selection) -> Option<&mut Self> {
        let mut cur = self;
        for i in &(sel.0) {
            match cur {
                TodoItem::Task(_, _) => {
                    return None;
                }
                TodoItem::Group(_, xs) => match xs.get_mut(*i as usize) {
                    Some(x) => cur = x,
                    None => {
                        return None;
                    }
                },
            }
        }
        Some(cur)
    }
    fn get_prior(&self, sel: &Selection) -> Option<&Self> {
        if sel.0.len() == 0 {return Some(self);}
        let mut cur = self;
        for i in 0 .. sel.0.len() - 1 {
            let i = sel.0[i];
            match cur {
                TodoItem::Task(_, _) => {
                    return None;
                }
                TodoItem::Group(_, xs) => match xs.get(i as usize) {
                    Some(x) => cur = x,
                    None => {
                        return None;
                    }
                },
            }
        }
        Some(cur)
    }
    fn get_prior_mut(&mut self, sel: &Selection) -> Option<&mut Self> {
        let mut cur = self;
        for i in 0..sel.0.len() - 1 {
            let i = sel.0[i];
            match cur {
                TodoItem::Task(_, _) => {
                    return None;
                }
                TodoItem::Group(_, xs) => match xs.get_mut(i as usize) {
                    Some(x) => cur = x,
                    None => {
                        return None;
                    }
                },
            }
        }
        Some(cur)
    }
    fn bound(&self, sel: &Selection) -> bool {
        self.get(sel).is_some()
    }
    fn insert(&mut self, value: Self) {
        match self {
            TodoItem::Group(_, xs) => xs.push(value),
            TodoItem::Task(_, msg) => {
                *self = TodoItem::Group(msg.to_string(), vec![value]);
            }
        }
    }
    fn is_group(&self) -> bool {
        match self {
            TodoItem::Group(_, _) => true,
            _ => false,
        }
    }
    fn check_move(&self, sel: &Selection, action: CursMove) -> bool {
        match action {
            CursMove::Down => self.get_prior(sel).and_then(|x| {
                match x {
                    TodoItem::Task(_, _) => None,
                    TodoItem::Group(_, xs) => {
                        if sel.0.len() == 0 { return None; }
                        let prior_ind = sel.0[sel.0.len() - 1] as usize;
                        if prior_ind + 1 < xs.len() {Some(())} else {None}
                    }
                }
            }).is_some(),
            CursMove::Up   => self.get_prior(sel).and_then(|x| {
                if sel.0.len() == 0 { return None; }
                let prior_ind = sel.0[sel.0.len() - 1] as usize;
                if prior_ind > 0 {Some(())} else {None}
            }).is_some(),
            CursMove::Out  => self.get_prior(sel).is_some(),
            CursMove::In   => self.get(sel)
                                  .and_then(|x|if x.is_group() {Some(())} else {None})
                                  .is_some(),
        }
    }
    fn do_move(&self, sel: &mut Selection, action: CursMove) {
        if self.check_move(sel, action) {
            sel.do_move(action);
        }
    }
}

#[derive(Clone)]

struct Selection(Vec<u8>);
// selected is `depth == Selection.0.len()`
impl Selection {
    fn do_move(&mut self, action: CursMove) {
        // Assumes that it can move
        let clen = self.0.len();
        match action {
            CursMove::Down => self.0[ clen - 1] += 1,
            CursMove::Up => self.0[clen - 1] -= 1,
            CursMove::Out => {self.0.pop();},
            CursMove::In => self.0.push(0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum CursMove {
    In,
    Out,
    Down,
    Up,
}

mod llywterf;
fn main() -> Result<()> {
    println!("creating llywterf instance");
    let mut terf = llywterf::TerfLleol::newidd(stdout(), stdin())?;
    println!("setting llywterf");
    terf.newid().canon(false).echo(false).llawnsgrin(true).atod()?;
    println!("Continuing");

    let mut test = TodoItem::Group(
        String::from("test 1"),
        vec![
            TodoItem::Group(String::from("test 1.1"), vec![
                            TodoItem::Task(false, String::from("test 1.1.1")),
                            TodoItem::Task(false, String::from("test 1.1.2")),
                        ]),
            TodoItem::Task(false, String::from("test 1.2")),
            TodoItem::Group(String::from("test 1.3"), vec![
                            TodoItem::Task(false, String::from("test 1.3.1")),
                            TodoItem::Task(false, String::from("test 1.3.2")),
                        ]),
        ],
    );

    let mut sel = Selection(vec![]);

    'testo: {
        /*
        println!("Testing arbed and lwytho");
        let mut buff = Vec::<u8>::with_capacity(32);
        test.arbed(&mut buff)?;
        println!("llwytho object");
        let mut out: MaybeUninit<TodoItem> = MaybeUninit::uninit();
        unsafe { out.assume_init_mut().llwytho(&mut VecRead::new(buff))?; }
        let out = unsafe { out.assume_init() };
        out.render(1, &mut stdout(), None)?;
        */
        todo!("Re-start saving tests");
    }
    terf.ungell()?;
    
    loop {
        println!("\x1b[2J\x1b[1;1H");
        test.render(1, &mut stdout(), Some(&sel))?;
        let lth = terf.ungell()?;
        if lth.is_none() {break;}
        match lth.unwrap() {
            'q' => break,
            ' ' => {test.get_mut(&sel).map(|x| x.complete(!x.completed()));},
            'h' => test.do_move(&mut sel, CursMove::Out),
            'l' => test.do_move(&mut sel, CursMove::In),
            'j' => test.do_move(&mut sel, CursMove::Down),
            'k' => test.do_move(&mut sel, CursMove::Up),
            'J' => 'round: {
                // in, down, out'n'down
                if test.check_move(&sel, CursMove::In)   {sel.do_move(CursMove::In);   break 'round;}
                if test.check_move(&sel, CursMove::Down) {sel.do_move(CursMove::Down); break 'round;}
                if test.check_move(&sel, CursMove::Out)  {
                    let save = Selection(sel.0.clone());
                    sel.do_move(CursMove::Out);
                    if !test.check_move(&sel, CursMove::Down) {
                        sel = save;
                        break 'round;
                    }
                    sel.do_move(CursMove::Down);
                    break 'round;
                }
            }
            'K' => 'round: {
                // out, up 
                if test.check_move(&sel, CursMove::Up) {
                    sel.do_move(CursMove::Up);
                    while test.check_move(&sel, CursMove::In) {
                        sel.do_move(CursMove::In);
                        while test.check_move(&sel, CursMove::Down) {
                            sel.do_move(CursMove::Down);
                        }
                    }
                    break 'round;
                }
                if test.check_move(&sel, CursMove::Out) {sel.do_move(CursMove::Out); break 'round;} 
            }
            'i' => {
                print!("\x1b[H\x1b[2K\x1b[0m> ");
                stdout().flush()?;
                let mut buff = String::with_capacity(16);
                terf.newid().echo(true).canon(true).atod()?;
                let l = stdin().read_line(&mut buff)?;
                buff.truncate(l - 1);
                test.get_mut(&sel).map( move |x|x.insert(TodoItem::Task(false, buff)));
                terf.newid().echo(false).canon(false).atod()?;
                /* get input, trim, insert */
            }
            'w' => {
                print!("\x1b[H\x1b[2K\x1b[0m> ");
                stdout().flush()?;
                let mut buff = String::with_capacity(16);
                terf.newid().echo(true).canon(true).atod()?;
                let l = stdin().read_line(&mut buff)?;
                buff.truncate(l - 1);
                terf.newid().echo(false).canon(false).atod()?;
            }
            _ => (),
        }
    }
    Ok(())
}

trait Arbed where Self: Sized {
    fn arbed<W: Write>(&self, allbwn: &mut W) -> Result<()>;
    fn llwytho<R: Read>(whr: &mut MaybeUninit<Self>, mewnbwn: &mut R) -> Result<()>;
}
impl Arbed for usize  {
    fn arbed<W: Write>(&self, allbwn: &mut W) -> Result<()> {
        allbwn.write(&self.to_le_bytes()).and(Ok(()))
    }
    fn llwytho< R: Read>(whr: &mut MaybeUninit<Self>, mewnbwn: &mut R) -> Result<()> {
        let mut buff: [u8; size_of::<Self>()] = [0; size_of::<Self>()];
        mewnbwn.read_exact(&mut buff)?;
        whr.write(usize::from_le_bytes(buff));
        Ok(())
    }
}
impl Arbed for bool {
    fn arbed<W: Write>(&self, allbwn:&mut W) -> Result<()> {
        let buff = (if *self {1} else {0} as u8).to_le_bytes();
        allbwn.write(&buff)?;
        Ok(())
    }
    fn llwytho<R: Read>(whr: &mut MaybeUninit<Self>, mewnbwn: &mut R) -> Result<()> {
        let mut buff: [u8; 1] = [ 0 ];
        mewnbwn.read_exact(&mut buff)?;
        whr.write(u8::from_le_bytes(buff) != 0);
        Ok(())
    }
}
impl<T: Arbed> Arbed for Vec<T> {
    fn arbed<W: Write>(&self, allbwn: &mut W) -> Result<()> {
        todo!("Implement");
    }
    fn llwytho<R: Read>(whr: &mut MaybeUninit<Self>, mewnbwn: &mut R) -> Result<()> {
        todo!("Implement");
    }
}

struct VecRead<T>(Vec<T>, usize);
impl<T> VecRead<T> { fn new(inp: Vec<T>) -> Self { VecRead(inp, 0) } }
impl Read for VecRead<u8> {
    fn read(&mut self, targ: &mut [u8]) -> Result<usize> {
        if targ.len() <= self.0.len() - self.1 {
            for i in self.1 .. self.1 + targ.len() {
                targ[i - self.1] = self.0[i];
            }
            self.1 += targ.len();
            return Ok(targ.len());
        }
        let l = self.0.len() - self.1;
        for i in 0 .. l {
            targ[i] = self.0[self.1 + i];
        }
        Ok(l)
    } 
}
