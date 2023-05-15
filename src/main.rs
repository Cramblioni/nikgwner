use std::{io::{stdin, stdout, Result, Write},};
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
        let selected = if let Some(Selection::Termin) = sel {
            true
        } else {
            false
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
                } else if let Some(Selection::Index(ind, nxt)) = sel {
                    if *ind == i as u8 {
                        Some(nxt.as_ref())
                    } else {
                        None
                    }
                } else {
                    None
                };
                x.render(depth + 1, outp, fsel)?;
            }
        }
        if selected {
            outp.write("\x1b[0m".as_bytes())?;
        }
        Ok(())
    }
    fn get(&self, sel: &Selection) -> Option<&Self> {
        if let Selection::Termin = sel {
            Some(self)
        } else if let Selection::Index(ind, nxt) = sel {
            if let TodoItem::Group(_, xs) = self {
                xs.get(*ind as usize).and_then(|x| x.get(nxt))
            } else {
                None
            }
        } else {
            None
        }
    }
    fn get_mut(&mut self, sel: &Selection) -> Option<&mut Self> {
        if let Selection::Termin = sel {
            Some(self)
        } else if let Selection::Index(ind, nxt) = sel {
            if let TodoItem::Group(_, xs) = self {
                xs.get_mut(*ind as usize).and_then(|x| x.get_mut(nxt))
            } else {
                None
            }
        } else {
            None
        }
    }
    fn get_prior(&self, sel: &Selection) -> Option<&Self> {
        match sel {
            Selection::Termin => None,
            Selection::Index(ind, x) => match x.as_ref() {
                Selection::Termin => Some(self),
                Selection::Index(_, nxt) => match self {
                    Self::Task(_, _) => None,
                    Self::Group(_, xs) => xs.get(*ind as usize).and_then(|x| x.get_prior(nxt)),
                },
            },
        }
    }
    fn get_prior_mut(&mut self, sel: &Selection) -> Option<&mut Self> {
        match sel {
            Selection::Termin => None,
            Selection::Index(ind, x) => match x.as_ref() {
                Selection::Termin => Some(self),
                Selection::Index(_, nxt) => match self {
                    Self::Task(_, _) => None,
                    Self::Group(_, xs) => {
                        xs.get_mut(*ind as usize).and_then(|x| x.get_prior_mut(nxt))
                    }
                },
            },
        }
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
    fn check_move(&self, sel: &Selection, action: CursMove) -> bool {
        let prior = self.get_prior(sel);
        let select = self.get(sel);
        let sprior = sel.get_prior();
        match action {
            CursMove::Out => prior.is_some(),
            CursMove::In => {
                select.is_some()
                    & match select.unwrap() {
                        TodoItem::Group(_, _) => true,
                        _ => false,
                    }
            }
            CursMove::Up => {
                if sprior.is_none() {
                    return false;
                }
                let sprior = sprior.unwrap();
                // logic
                match sprior {
                    Selection::Termin => false,
                    Selection::Index(i, _) => *i > 0,
                }
            }
            CursMove::Down => {
                if prior.is_none() {
                    return false;
                }
                let prior = prior.unwrap();
                if sprior.is_none() {
                    return false;
                }
                let sprior = sprior.unwrap();
                match prior {
                    TodoItem::Task(_, _) => false,
                    TodoItem::Group(_, xs) => {
                        // logic
                        match sprior {
                            Selection::Termin => false,
                            Selection::Index(i, _) => !((*i + 1) as usize >= xs.len()),
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
enum Selection {
    Termin,
    Index(u8, Box<Selection>),
}
impl Selection {
    fn get_mut(&mut self) -> &mut Self {
        match self {
            Selection::Termin => self,
            Selection::Index(_, xs) => xs.get_mut(),
        }
    }
    fn get_prior(&self) -> Option<&Self> {
        match self {
            Selection::Termin => None,
            &Selection::Index(_, ref nxt) => match nxt.as_ref() {
                Selection::Termin => Some(self),
                _ => nxt.as_ref().get_prior(),
            },
        }
    }
    fn get_prior_mut(&mut self) -> Option<&mut Self> {
        if let Selection::Termin = self {
            return None;
        }
        if let Selection::Index(_, nxt) = self {
            return nxt.as_mut().get_prior_mut();
        } else {
            return Some(self);
        }
    }
    fn do_move(&mut self, action: CursMove) {
        match action {
            CursMove::In => {
                *self.get_mut() = Selection::Index(0, Box::new(Selection::Termin));
            }
            CursMove::Out => {
                if let Some(x) = self.get_prior_mut() {
                    *x = Selection::Termin
                };
            }
            CursMove::Up => {
                if let Some(x) = self.get_prior_mut() {
                    let mut temp = Selection::Termin;
                    std::mem::swap(x, &mut temp);
                    match temp {
                        Selection::Termin => (),
                        Selection::Index(i, nxt) => {
                            temp = Selection::Index(i - 1, nxt);
                        }
                    }
                    std::mem::swap(x, &mut temp);
                }
            }
            CursMove::Down => {
                if let Some(x) = self.get_prior_mut() {
                    let mut temp = Selection::Termin;
                    std::mem::swap(x, &mut temp);
                    match temp {
                        Selection::Termin => (),
                        Selection::Index(i, nxt) => {
                            temp = Selection::Index(i + 1, nxt);
                        }
                    }
                    std::mem::swap(x, &mut temp);
                }
            }
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
    terf.canon(false).echo(false).llawnsgrin(true).atod()?;
    println!("Continuing");

    let mut test = TodoItem::Group(
        String::from("test 1"),
        vec![
            TodoItem::Task(true, String::from("test 1.1")),
            TodoItem::Task(false, String::from("test 1.2")),
        ],
    );
    let mut sel = Selection::Index(1, Box::new(Selection::Termin));
    println!("Hello, world!");
    test.render(1, &mut stdout(), None)?;
    println!("Checking if selection in bounds\n\t= {}", test.bound(&sel));
    let _ = terf.ungell()?;

    println!("\x1b[2J\x1b[1;1Hmutating - completing test 1.2");
    let test2 = test.get_mut(&sel).expect("Malimple error");
    test2.complete(true);
    test.render(1, &mut stdout(), Some(&sel))?;
    let _ = terf.ungell()?;

    println!("\x1b[2J\x1b[1;1Hmutating - uncompleting test 1");
    let test2 = test.get_mut(&Selection::Termin).expect("Malimple error");
    test2.complete(false);
    test.render(1, &mut stdout(), Some(&sel))?;
    let _ = terf.ungell()?;

    println!("\x1b[2J\x1b[1;1Hmutating - inserting test 1.3");
    let test2 = test.get_mut(&Selection::Termin).expect("fuck");
    test2.insert(TodoItem::Task(false, "test 1.3".to_string()));
    test.render(1, &mut stdout(), Some(&sel))?;
    let _ = terf.ungell()?;

    println!("\x1b[2J\x1b[1;1Hmutating - setting test 1 and inserting test 1.2.1");
    let test2 = test.get_mut(&Selection::Termin).expect("Malimple error");
    test2.complete(true);
    let test2 = test.get_mut(&sel).expect("fuck");
    test2.insert(TodoItem::Task(false, "test 1.2.1".to_string()));
    test.render(1, &mut stdout(), Some(&sel))?;
    let _ = terf.ungell()?;

    println!("\x1b[2J\x1b[1;1HRendering just test 1.2");
    test.get(&sel)
        .expect("bounds error")
        .render(1, &mut stdout(), Some(&Selection::Termin))?;

    println!("\x1b[2J\x1b[1;1Hmutating - getting node prior test 1.2 and completing");
    let test2 = test.get_prior_mut(&sel).expect("Malimple error");
    test2.complete(true);
    test.render(1, &mut stdout(), Some(&sel))?;
    let _ = terf.ungell()?;

    println!("\x1b[2J\x1b[1;1HMoving Cursor out");
    if !test.check_move(&sel, CursMove::Out) {
        println!("check_move failed");
    } else {
        println!("check_move successful");
        sel.do_move(CursMove::Out);
    }
    test.render(1, &mut stdout(), Some(&sel))?;
    let _ = terf.ungell()?;

    Ok(())
}
