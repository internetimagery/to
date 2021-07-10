use cpython::{
    exc::TypeError, py_class, py_exception, py_module_initializer, ObjectProtocol, PyClone, PyDrop,
    PyErr, PyObject, PyResult, PySequence, PythonObject,
};
use search::Graph;
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::convert::TryInto;
mod search;

#[cfg(feature = "python2")]
pub type Int = std::os::raw::c_long;

#[cfg(feature = "python3")]
pub type Int = isize;

// Simple utility, make a hash set out of python sequence
macro_rules! hash_seq {
    ($py:expr, $seq:expr) => {
        $seq.iter($py)?
            .filter_map(|v| match v {
                Ok(v) => v.hash($py).ok(),
                _ => None,
            })
            .collect()
    };
}
// Small utility to log using python logger.
macro_rules! warn {
    ($py:expr, $message:expr) => {
        $py.import("logging")?
            .call($py, "getLogger", ("to",), None)?
            .call_method($py, "warning", (&$message,), None)?;
    };
}

//////////////////////////////////////////////////
// MODULE SETUP
// NOTE: "_internal" is the name of this module after build process moves it
py_module_initializer!(_internal, |py, m| {
    m.add(py, "__doc__", "Simple plugin based A to B function chaining.
        You could be wanting to convert between a chain of types, or traverse a bunch of object oriented links.
        If you're often thinking \"I have this, how can I get that\", then this type of solution could help.

        >>> conv = Conversions()
        >>> conv.add_conversion(1, str, [\"url\"], WebPage, [], load_webpage)
        >>> conv.add_revealer(str, http_revealer) # optional convenience
        >>> conv.convert(\"http://somewhere.html\", WebPage)
    ")?;
    m.add(py, "ConversionError", py.get_type::<ConversionError>())?;
    m.add_class::<Conversions>(py)?;
    Ok(())
});
//////////////////////////////////////////////////

//////////////////////////////////////////////////
// Exceptions
py_exception!(to, ConversionError); // Triggered when errors occurred during conversion process
                                    //////////////////////////////////////////////////

py_class!(class Conversions |py| {
    data graph: RefCell<Graph<Int, Int, Int>>;
    data functions: RefCell<HashMap<Int, PyObject>>;
    data revealers: RefCell<HashMap<Int, Vec<PyObject>>>;
    def __new__(_cls) -> PyResult<Conversions> {
        Conversions::create_instance(
            py,
            RefCell::new(Graph::new()),
            RefCell::new(HashMap::new()),
            RefCell::new(HashMap::new()),
        )
    }

    /// Add a function so it may be used as a step in the composition process later.
    /// Eventually a composition chain will consist of a number of these functions placed back to back.
    /// So the simpler, smaller and more focused the function the better.
    ///
    /// Args:
    ///     cost (int):
    ///         A number representing how much work this function needs to do.
    ///         Lower numbers are prioritized. This lets the composition prefer the cheapest option.
    ///         eg: just getting an attribute would be a low number. Accessing an network service would be higher etc
    ///     type_in (Type[A]):
    ///         Type of input expected.
    ///         eg str / MyClass or a composite type eg frozenset([Type1, Type2])
    ///     variations_in (Sequence[Hashable]):
    ///         A sequence of hashable "tags" further describing the input type.
    ///         For the node to be used, all these variations are required (dependencies).
    ///         This is useful if the more simple type is not enough by itself.
    ///         eg: str (can be path/url/email/name/etc)
    ///     type_out (Type[B]):
    ///         Same as "type_in", but representing the output of the transmutation.
    ///     variations_out (Sequence[Hashable]):
    ///         Same as "variations_in" except that variations are descriptive and not dependencies.
    ///         They can satisfy dependencies for transmuters further down the chain.
    ///     function (Callable[[A], B]):
    ///         The converter itself. Take a single input, produce a single output.
    ///         It is important that only an simple conversion is made, and that any deviation is raised as an Error.
    ///         eg: maybe some attribute is not available and usually you'd return None. There is no strict type
    ///         checking here, so raise an error and bail instead.
    def add_conversion(
        &self,
        cost: Int,
        type_in: &PyObject,
        variations_in: &PySequence,
        type_out: &PyObject,
        variations_out: &PySequence,
        function: PyObject
    ) -> PyResult<PyObject> {
        let hash_in = type_in.hash(py)?;
        let hash_out = type_out.hash(py)?;
        let hash_func = function.hash(py)?;
        let hash_var_in = hash_seq!(py, variations_in);
        let hash_var_out = hash_seq!(py, variations_out);

        // Store a reference to the python object in this outer layer
        // but refer to it via its hash.
        self.functions(py).borrow_mut().insert(hash_func, function);
        self.graph(py).borrow_mut().add_edge(
            cost.try_into().expect("Cost needs to be an int"), hash_in, hash_var_in, hash_out, hash_var_out, hash_func,
        );
        Ok(py.None())
    }

    /// Supply a function that will attempt to reveal insights into the provided data as variations.
    /// This is a convenience aid, to assist in detecting input variations automatically so they do not
    /// need to be expicitly specified.
    /// The activator function should run quickly so as to keep the entire process smooth.
    /// ie simple attribute checks, string regex etc
    ///
    /// Important note: These functions are not run on intermediate conversions, but only on the
    /// supplied data.
    ///
    /// Args:
    ///     type_in (Type[A]):
    ///         The type of input this function accepts.
    ///     function (Callable[[A], Iterator[Hashable]]):
    ///         Function that takes the value provided (of the type above) and yields any variations it finds.
    ///         eg: str type could check for link type if the string is http://something.html and
    ///         yield "protocol" "http" "url"
    def add_revealer(&self, type_in: &PyObject, function: PyObject) -> PyResult<PyObject> {
        self.revealers(py).borrow_mut().entry(type_in.hash(py)?).or_insert(Vec::new()).push(function);
        Ok(py.None())
    }

    /// From a given type, attempt to produce a requested type.
    /// OR from some given data, attempt to traverse links to get the requested data.
    ///
    /// Args:
    ///     value (Any): The input you have going into the process. This can be anything.
    ///     type_want (Type[B]):
    ///         The type you want to recieve. A chain of converters will be produced
    ///         attempting to attain this type.
    ///     variations_want (Sequence[Hashable]):
    ///         A sequence of variations further describing the type you wish to attain.
    ///         This is optional but can help guide the selection of converters through more complex transitions.
    ///     type_have (Type[A]):
    ///         An optional override for the starting type.
    ///         If not provided the type of the value is taken instead.
    ///     variations_have (Sequence[Hashable]):
    ///         Optionally include any extra variations to the input.
    ///         If context is known but hard to detect this can help direct a more complex
    ///         transmutation.
    ///     explicit (bool):
    ///         If this is True, the "variations_have" argument will entirely override
    ///         any detected tags. Enable this to use precisesly what you specify (no automatic detection).
    /// Returns:
    ///     B: Whatever the result requested happens to be
    def convert(
        &self,
        value: PyObject,
        type_want: &PyObject,
        variations_want: Option<&PySequence> = None,
        type_have: Option<&PyObject> = None,
        variations_have: Option<&PySequence> = None,
        explicit: bool = false,
        debug: bool = false,
    ) -> PyResult<PyObject> {
        let hash_in = match type_have {
            Some(type_override) => type_override.hash(py)?,
            None => value.get_type(py).into_object().hash(py)?
        };
        let hash_out = type_want.hash(py)?;
        let hash_var_out = match variations_want {
            Some(vars) => hash_seq!(py, vars),
            None => BTreeSet::new(),
        };
        let mut hash_var_in = match variations_have {
            Some(vars) => hash_seq!(py, vars),
            None => BTreeSet::new(),
        };

        // Short circut if we are just looking at the same thing
        if hash_in == hash_out && hash_var_in == hash_var_out {
            return Ok(value)
        }

        if !explicit {
            // We don't want to be explicit, so
            // run the activator to detect initial variations
            if let Some(funcs) = self.revealers(py).borrow().get(&hash_in) {
                for func in funcs {
                    for variation in func.call(py, (value.clone_ref(py),), None)?.iter(py)? {
                        hash_var_in.insert(variation?.hash(py)?);
                    }
                }
            }
        }

        // Retry a few times, if something breaks along the way.
        // Collect errors.
        // If we run out of paths to take or run out of reties,
        // and there are still errors. Raise with info from all of them.
        let mut skip_edges = BTreeSet::new();
        let mut errors = Vec::new();
        'outer: for _ in 0..10 {
            if let Some(edges) = self.graph(py).borrow().search(hash_in, &hash_var_in, hash_out, &hash_var_out, &skip_edges) {
                let functions = self.functions(py).borrow();
                let mut result = value.clone_ref(py);
                for edge in edges {
                    let func = functions.get(&edge.data).expect("Function is there");
                    if debug {
                        warn!(py, format!("{}({}) -> ...", func.to_string(), result.to_string()));
                    }
                    match func.call(py, (result,), None) {
                        Ok(res) => {
                            if debug {
                                warn!(py, format!("... -> {}", res.to_string()));
                            }
                            result = res;
                        },
                        Err(mut err) => {
                            let message = format!(
                                    "{}: {}",
                                    err.get_type(py).name(py),
                                    err.instance(py).str(py)?.to_string(py)?,
                                );
                            warn!(py, message);
                            errors.push(message);
                        // Ignore these when trying again.
                        // This allows some level of failure
                        // and with enough edges perhaps we
                        // can find another path.
                        skip_edges.insert(edge);
                        continue 'outer
                        }
                    };
                }
                return Ok(result)
            }
            break
        }
        if errors.len() != 0 {
            Err(PyErr::new::<ConversionError, _>(py, format!(
                "Some problems occurred during the conversion process:\n{}",
                errors.join("\n")
                )))
        } else {
            Err(PyErr::new::<TypeError, _>(
                py, format!(
                    "Could not convert {} to {}. Perhaps some conversion steps are missing.",
                    value, type_want
                )))
        }
    }

    ///////////////////////////////////////////////////////////////
    // Satisfy python garbage collector
    // because we hold a reference to some functions provided
    def __traverse__(&self, visit) {
        for function in self.functions(py).borrow().values() {
            visit.call(function)?;
        }
        for functions in self.revealers(py).borrow().values() {
            for function in functions {
                visit.call(function)?;
            }
        }
        Ok(())
    }

    def __clear__(&self) {
        for (_, func) in self.functions(py).borrow_mut().drain() {
            func.release_ref(py);
        }
        for (_, mut funcs) in self.revealers(py).borrow_mut().drain() {
            for func in funcs.drain(..) {
                func.release_ref(py);
            }
        }
    }
    ///////////////////////////////////////////////////////////////
});
