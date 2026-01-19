use crate::analytics::SmileGraph;

// The parameters that define the SVI smile curve function
#[derive(serde::Deserialize, serde::Serialize)]
pub struct SVICurveParameters {
    a: f64,
    b: f64,
    p: f64,
    m: f64,
    o: f64,
}

impl SVICurveParameters {
    /// Create a new and empty instance (everything set to 0).
    pub fn new_empty() -> SVICurveParameters {
        let params: SVICurveParameters = SVICurveParameters {
            a: 0.0,
            b: 0.0,
            p: 0.0,
            m: 0.0,
            o: 0.0,
        };

        Self::assert_valid(&params);

        params
    }

    pub fn new_from_values(a: f64, b: f64, p: f64, m: f64, o: f64) -> SVICurveParameters {
        let params: SVICurveParameters = SVICurveParameters {
            a: a,
            b: b,
            p: p,
            m: m,
            o: o,
        };

        Self::assert_valid(&params);

        params
    }

    pub fn set_params(&mut self, a: f64, b: f64, p: f64, m: f64, o: f64) {
        self.a = a;
        self.b = b;
        self.p = p;
        self.m = m;
        self.o = o;

        Self::assert_valid(&self);
    }

    pub fn get_a(&self) -> f64 {
        self.a
    }

    pub fn get_b(&self) -> f64 {
        self.b
    }

    pub fn get_p(&self) -> f64 {
        self.p
    }

    pub fn get_m(&self) -> f64 {
        self.m
    }

    pub fn get_o(&self) -> f64 {
        self.o
    }

    fn assert_valid(params: &Self) {}
}

#[derive(PartialEq, Copy, Clone)]
pub enum OptionType {
    Call = 1,
    Put = 2,
}

impl OptionType {
    pub fn from_string(option_type: &str) -> OptionType {
        match option_type.to_ascii_lowercase().as_str() {
            "call" => OptionType::Call,
            "put" => OptionType::Put,
            _ => panic!("Invalid option type {option_type}"),
        }
    }
}

/// Used to store the smile graph data to file.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct SmileGraphsDataContainer {
    pub smile_graphs: Vec<SmileGraph>,
}
