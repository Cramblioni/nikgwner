use std::io::{stdin, stdout, Result, Write};
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
        let mut cur = self;
        for i in 0..sel.0.len() - 1 {
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
                        let prior_ind = sel.0[sel.0.len() - 1] as usize;
                        if prior_ind + 1 < xs.len() {Some(())} else {None}
                    }
                }
            }).is_some(),
            CursMove::Up   => self.get_prior(sel).and_then(|x| {
                let prior_ind = sel.0[sel.0.len() - 1] as usize;
                if prior_ind > 0 {Some(())} else {None}
            }).is_some(),
            CursMove::Out  => self.get_prior(sel).is_some(),
            CursMove::In   => self.get(sel)
                                  .and_then(|x|if x.is_group() {Some(())} else {None})
                                  .is_some(),
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
fn steps() -> Result<()> {
    println!("creating llywterf instance");
    let mut terf = llywterf::TerfLleol::newidd(stdout(), stdin())?;
    println!("setting llywterf");
    terf.canon(false).echo(false).llawnsgrin(true).atod()?;
    println!("Continuing");

    let mut test = TodoItem::Group(
        String::from("test 1"),
        vec![
            TodoItem::Task(true, String::from("test 1.1")),
            TodoItem::Task(false, String::from("test 1.2")),
        ],
    );
    let mut sel = Selection(vec![1]);
    println!("\x1b[2J\x1b[1;1HHello, world!");
    test.render(1, &mut stdout(), None)?;
    println!("Checking if selection in bounds\n\t= {}", test.bound(&sel));
    let _ = terf.ungell()?;

    println!("\x1b[2J\x1b[1;1Hmutating - completing test 1.2");
    let test2 = test.get_mut(&sel).expect("Malimple error");
    test2.complete(true);
    test.render(1, &mut stdout(), Some(&sel))?;
    let _ = terf.ungell()?;

    println!("\x1b[2J\x1b[1;1Hmutating - uncompleting test 1");
    let test2 = test.get_mut(&Selection(vec![])).expect("Malimple error");
    test2.complete(false);
    test.render(1, &mut stdout(), Some(&sel))?;
    let _ = terf.ungell()?;

    println!("\x1b[2J\x1b[1;1Hmutating - inserting test 1.3");
    let test2 = test.get_mut(&Selection(vec![])).expect("fuck");
    test2.insert(TodoItem::Task(false, "test 1.3".to_string()));
    test.render(1, &mut stdout(), Some(&sel))?;
    let _ = terf.ungell()?;

    println!("\x1b[2J\x1b[1;1Hmutating - setting test 1 and inserting test 1.2.1");
    let test2 = test.get_mut(&Selection(vec![])).expect("Malimple error");
    test2.complete(true);
    let test2 = test.get_mut(&sel).expect("fuck");
    test2.insert(TodoItem::Task(false, "test 1.2.1".to_string()));
    test.render(1, &mut stdout(), Some(&sel))?;
    let _ = terf.ungell()?;

    println!("\x1b[2J\x1b[1;1HRendering just test 1.2");
    test.get(&sel)
        .expect("bounds error")
        .render(1, &mut stdout(), Some(&Selection(vec![])))?;

    println!("\x1b[2J\x1b[1;1Hmutating - getting node prior test 1.2 and completing");
    let test2 = test.get_prior_mut(&sel).expect("Malimple error");
    test2.complete(true);
    test.render(1, &mut stdout(), Some(&sel))?;
    let _ = terf.ungell()?;

    println!("\x1b[2J\x1b[1;1Hmoving - in");
    if test.check_move(&sel, CursMove::In) {
        println!("check_move success");
        sel.do_move(CursMove::In);
    } else {
        println!("check_move failure");
    }
    test.render(1, &mut stdout(), Some(&sel))?;
    let _ = terf.ungell()?;

    println!("\x1b[2J\x1b[1;1Hmoving - out");
    if test.check_move(&sel, CursMove::Out) {
        println!("check_move success");
        sel.do_move(CursMove::Out);
    } else {
        println!("check_move failure");
    }
    test.render(1, &mut stdout(), Some(&sel))?;
    let _ = terf.ungell()?;

    println!("\x1b[2J\x1b[1;1Hmoving - down");
    if test.check_move(&sel, CursMove::Down) {
        println!("check_move success");
        sel.do_move(CursMove::Down);
    } else {
        println!("check_move failure");
    }
    test.render(1, &mut stdout(), Some(&sel))?;
    let _ = terf.ungell()?;

    println!("\x1b[2J\x1b[1;1Hmoving - up");
    if test.check_move(&sel, CursMove::Up) {
        println!("check_move success");
        sel.do_move(CursMove::Up);
    } else {
        println!("check_move failure");
    }
    test.render(1, &mut stdout(), Some(&sel))?;
    let _ = terf.ungell()?;

    Ok(())
}
