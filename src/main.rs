use std::io::{stdin, stdout, Result};
use std::io::{Read, BufRead, Write, Error, ErrorKind};
use std::mem::{size_of};
use std::os::fd::AsRawFd;
use std::fs::File;

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
        if sel.0.len() == 0 {return None}
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
            CursMove::Up   => self.get_prior(sel).and_then(|_| {
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

    fn delete(&mut self, sel: &Selection) -> Option<()> {
        let prev = self.get_prior_mut(sel)?;
        if let TodoItem::Group(_,xs) = prev {
            xs.remove(sel.get_end()? as usize);
        }
        Some(())
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
    fn get_end(&self) -> Option<u8> {
        self.0.get(self.0.len() - 1).copied()
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
    let mut terf = llywterf::TerfLleol::newidd(stdout(), stdin().lock())?;
    println!("setting llywterf");
    terf.newid().canon(false).echo(false).llawnsgrin(true).atod()?;
    println!("Continuing");
    let mut source = None;
    let mut cur_todo = TodoItem::Group(
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

    loop {
        println!("\x1b[2J\x1b[1;1H");
        write!(terf, "{}\n", source.as_ref().unwrap_or(&String::with_capacity(0)))?;
        while !cur_todo.bound(&sel) {sel.0.pop();}
        cur_todo.render(1, &mut stdout(), Some(&sel))?;
        let lth = terf.ungell()?;
        if lth.is_none() {break;}
        match lth.unwrap() {
            'q' => break,
            ' ' => {cur_todo.get_mut(&sel).map(|x| x.complete(!x.completed()));},
            'h' => cur_todo.do_move(&mut sel, CursMove::Out),
            'l' => cur_todo.do_move(&mut sel, CursMove::In),
            'j' => cur_todo.do_move(&mut sel, CursMove::Down),
            'k' => cur_todo.do_move(&mut sel, CursMove::Up),
            'J' => 'round: {
                // in, down, out'n'down
                if cur_todo.check_move(&sel, CursMove::In)   {sel.do_move(CursMove::In);   break 'round;}
                if cur_todo.check_move(&sel, CursMove::Down) {sel.do_move(CursMove::Down); break 'round;}
                if cur_todo.check_move(&sel, CursMove::Out)  {
                    let save = Selection(sel.0.clone());
                    sel.do_move(CursMove::Out);
                    if !cur_todo.check_move(&sel, CursMove::Down) {
                        sel = save;
                        break 'round;
                    }
                    sel.do_move(CursMove::Down);
                    break 'round;
                }
            }
            'K' => 'round: {
                // out, up
                if cur_todo.check_move(&sel, CursMove::Up) {
                    sel.do_move(CursMove::Up);
                    while cur_todo.check_move(&sel, CursMove::In) {
                        sel.do_move(CursMove::In);
                        while cur_todo.check_move(&sel, CursMove::Down) {
                            sel.do_move(CursMove::Down);
                        }
                    }
                    break 'round;
                }
                if cur_todo.check_move(&sel, CursMove::Out) {sel.do_move(CursMove::Out); break 'round;}
            }
            'i' => {
                let item = prompt(&mut terf)?;
                cur_todo.get_mut(&sel).map( move |x|x.insert(TodoItem::Task(false, item)));
                terf.newid().echo(false).canon(false).atod()?;
                /* get input, trim, insert */
            }
            'w' => {
                let path = prompt(&mut terf)?;
                let mut file = if !path.is_empty() {
                    let file = File::create(&path)?;
                    source.insert(path);
                    file
                } else {
                    File::create(&path)?
                };
                cur_todo.arbed(&mut file)?;
            },
            'W' => {
                let path = prompt(&mut terf)?;
                let mut file = File::open(&path)?;
                match TodoItem::llwytho(&mut file) {
                    Ok(nxt) => { 
                        cur_todo = nxt;
                        source.insert(path);
                    }
                    Err(e)  => {
                        terf.write(b"\x1b[H\x1b[2K\x1b[0m> ")?;
                        write!(terf, "{e}")?;
                    }
                }
            }
            'd' => {
                 cur_todo.delete(&sel);
            }
            _ => (),
        }
    }
    if let Some(source) = source {
        let mut file = File::create(source)?;
        cur_todo.arbed(&mut file)?;
    }
    Ok(())
}

fn prompt<O: Write + AsRawFd, I: Read + BufRead + AsRawFd>(terf: &mut llywterf::TerfLleol<O, I> ) -> Result<String> {
    terf.write(b"\x1b[H\x1b[2K\x1b[0m> ")?;
    terf.flush()?;
    let mut buff = String::with_capacity(16);
    terf.newid().echo(true).canon(true).atod()?;
    let l = terf.read_line(&mut buff)?;
    buff.truncate(l - 1);
    terf.newid().echo(false).canon(false).atod()?;
    return Ok(buff);
}


trait Arbed where Self: Sized {
    fn arbed<W: Write>(&self, allbwn: &mut W) -> Result<()>;
    fn llwytho<R: Read>(mewnbwn: &mut R) -> Result<Self>;
}
impl Arbed for u8 {
    fn arbed<W: Write>(&self, allbwn: &mut W) -> Result<()> {
        allbwn.write(&self.to_le_bytes()).and(Ok(()))
    }
    fn llwytho< R: Read>(mewnbwn: &mut R) -> Result<Self> {
        let mut buff: [u8; size_of::<Self>()] = [0; size_of::<Self>()];
        mewnbwn.read_exact(&mut buff)?;
        Ok(Self::from_le_bytes(buff))
    }
}
impl Arbed for u16 {
    fn arbed<W: Write>(&self, allbwn: &mut W) -> Result<()> {
        allbwn.write(&self.to_le_bytes()).and(Ok(()))
    }
    fn llwytho< R: Read>(mewnbwn: &mut R) -> Result<Self> {
        let mut buff: [u8; size_of::<Self>()] = [0; size_of::<Self>()];
        mewnbwn.read_exact(&mut buff)?;
        Ok(Self::from_le_bytes(buff))
    }
}
impl Arbed for bool {
    fn arbed<W: Write>(&self, allbwn:&mut W) -> Result<()> {
        let buff = (if *self {1} else {0} as u8).to_le_bytes();
        allbwn.write(&buff)?;
        Ok(())
    }
    fn llwytho<R: Read>(mewnbwn: &mut R) -> Result<Self> {
        let mut buff: [u8; 1] = [ 0 ];
        mewnbwn.read_exact(&mut buff)?;
        Ok(u8::from_le_bytes(buff) != 0)
    }
}
impl<T: Arbed> Arbed for Vec<T> {
    fn arbed<W: Write>(&self, allbwn: &mut W) -> Result<()> {
        (self.len() as u8).arbed(allbwn)?;
        for i in self { i.arbed(allbwn)?; }
        Ok(())
    }
    fn llwytho<R: Read>(mewnbwn: &mut R) -> Result<Self> {
        let mut temp = Vec::with_capacity(u8::llwytho(mewnbwn)? as usize);
        for _ in 0 .. temp.capacity() {
            temp.push(T::llwytho(mewnbwn)?)
        }
        Ok(temp)
    }
}

impl Arbed for String {
    fn arbed<W: Write>(&self, allbwn: &mut W) -> Result<()> {
        (self.len() as u16).arbed(allbwn)?;
        allbwn.write(self.as_bytes())?;
        Ok(())
    }
    fn llwytho<R: Read>(mewnbwn: &mut R) -> Result<Self> {
        let mut buff = vec![0u8; u16::llwytho(mewnbwn)? as usize];
        mewnbwn.read_exact(&mut buff)?;
        match String::from_utf8(buff) {
            Ok(msg) => Ok(msg),
            Err(e)  => Err(Error::new(ErrorKind::Other, e))
        }
    }
}

impl Arbed for TodoItem {
    fn arbed<W: Write>(&self, allbwn: &mut W) -> Result<()> {
        match self {
            TodoItem::Task(c, msg) => {
                u8::arbed(&0, allbwn)?;
                c.arbed(allbwn)?;
                msg.arbed(allbwn)?;
            }
            TodoItem::Group(msg, xs) => {
                u8::arbed(&1, allbwn)?;
                msg.arbed(allbwn)?;
                xs.arbed(allbwn)?;
            }
        }
        Ok(())
    }
    fn llwytho<R: Read>(mewnbwn: &mut R) -> Result<Self> {
        match u8::llwytho(mewnbwn)? {
            0 => {
                Ok(TodoItem::Task(bool::llwytho(mewnbwn)?, String::llwytho(mewnbwn)?))
            }
            1 => {
                Ok(TodoItem::Group(String::llwytho(mewnbwn)?, Vec::<TodoItem>::llwytho(mewnbwn)?))
            }
            x => panic!("couldn't llwytho TodoItem of id {x}"),
        }
    }
}
