const ANNOTATIONS = [
  annotationP2SHActivation,
  annotationSegWitActivated,
  annotationTaprootActivated,
  annotationBitcoinCore28,
]
const MOVING_AVERAGE_DAYS = MOVING_AVERAGE_7D
const NAMES = ["P2PK", "P2PKH", "P2WPKH", "P2MS", "P2SH", "P2WSH", "P2TR", "P2A", "Unknown"]
const PRECISION = 0
let START_DATE = new Date();
START_DATE.setFullYear(new Date().getFullYear() - 10);

const CSVs = [
  fetchCSV("/csv/date.csv"),
  fetchCSV("/csv/outputs_p2pk_sum.csv"),
  fetchCSV("/csv/outputs_p2pkh_sum.csv"),
  fetchCSV("/csv/outputs_p2wpkh_sum.csv"),
  fetchCSV("/csv/outputs_p2ms_sum.csv"),
  fetchCSV("/csv/outputs_p2sh_sum.csv"),
  fetchCSV("/csv/outputs_p2wsh_sum.csv"),
  fetchCSV("/csv/outputs_p2tr_sum.csv"),
  fetchCSV("/csv/outputs_p2a_sum.csv"),
  fetchCSV("/csv/outputs_unknown_sum.csv"),
]

function preprocess(input) {
  let data = {
    date: [],
    p2pk: [],
    p2pkh: [],
    p2wpkh: [],
    p2ms: [],
    p2sh: [],
    p2wsh: [],
    p2tr: [],
    p2a: [],
    unknown: []
  }

  // CSV indices
  const [dates,
    outputs_p2pk, outputs_p2pkh, outputs_p2wpkh, outputs_p2ms, outputs_p2sh, outputs_p2wsh,
    outputs_p2tr, outputs_p2a, outputs_unknown] = input;

  // Initialize cumulative counters
  let cumulative_p2pk = 0;
  let cumulative_p2pkh = 0;
  let cumulative_p2wpkh = 0;
  let cumulative_p2ms = 0;
  let cumulative_p2sh = 0;
  let cumulative_p2wsh = 0;
  let cumulative_p2tr = 0;
  let cumulative_p2a = 0;
  let cumulative_unknown = 0;

  for (let i = 0; i < dates.length; i++) {
    data.date.push(+(new Date(dates[i].date)))

    // Calculate cumulative sums
    cumulative_p2pk += parseFloat(outputs_p2pk[i]["outputs_p2pk_sum"]) || 0
    cumulative_p2pkh += parseFloat(outputs_p2pkh[i]["outputs_p2pkh_sum"]) || 0
    cumulative_p2wpkh += parseFloat(outputs_p2wpkh[i]["outputs_p2wpkh_sum"]) || 0
    cumulative_p2ms += parseFloat(outputs_p2ms[i]["outputs_p2ms_sum"]) || 0
    cumulative_p2sh += parseFloat(outputs_p2sh[i]["outputs_p2sh_sum"]) || 0
    cumulative_p2wsh += parseFloat(outputs_p2wsh[i]["outputs_p2wsh_sum"]) || 0
    cumulative_p2tr += parseFloat(outputs_p2tr[i]["outputs_p2tr_sum"]) || 0
    cumulative_p2a += parseFloat(outputs_p2a[i]["outputs_p2a_sum"]) || 0
    cumulative_unknown += parseFloat(outputs_unknown[i]["outputs_unknown_sum"]) || 0

    data.p2pk.push(cumulative_p2pk)
    data.p2pkh.push(cumulative_p2pkh)
    data.p2wpkh.push(cumulative_p2wpkh)
    data.p2ms.push(cumulative_p2ms)
    data.p2sh.push(cumulative_p2sh)
    data.p2wsh.push(cumulative_p2wsh)
    data.p2tr.push(cumulative_p2tr)
    data.p2a.push(cumulative_p2a)
    data.unknown.push(cumulative_unknown)
  }

  return data
}

const DATA_KEYS = ['p2pk', 'p2pkh', 'p2wpkh', 'p2ms', 'p2sh', 'p2wsh', 'p2tr', 'p2a', 'unknown']

function chartDefinition(d, movingAverage) {
  let option = multiLineChart(d, DATA_KEYS, NAMES, movingAverage, PRECISION, START_DATE, ANNOTATIONS);
  option.tooltip["order"] = "valueDesc";
  option.tooltip["valueFormatter"] = (v) => Math.round(v).toLocaleString();
  option.yAxis.axisLabel = {formatter: (v) => formatCount(v)};
  return option;
}
