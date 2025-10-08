const ANNOTATIONS = [
  annotationP2SHActivation,
  annotationSegWitActivated,
  annotationTaprootActivated,
  annotationBitcoinCore28,
]
const MOVING_AVERAGE_DAYS = MOVING_AVERAGE_7D
const NAMES = ["P2PK", "P2PKH", "P2WPKH", "P2MS", "P2SH", "P2WSH", "P2TR (keypath)", "P2TR (scriptpath)", "P2A", "Unknown"]
const PRECISION = 0
let START_DATE = new Date();
START_DATE.setFullYear(new Date().getFullYear() - 10);

const CSVs = [
  fetchCSV("/csv/date.csv"),
  fetchCSV("/csv/inputs_p2pk_sum.csv"),
  fetchCSV("/csv/inputs_p2pkh_sum.csv"),
  fetchCSV("/csv/inputs_p2wpkh_sum.csv"),
  fetchCSV("/csv/inputs_p2ms_sum.csv"),
  fetchCSV("/csv/inputs_p2sh_sum.csv"),
  fetchCSV("/csv/inputs_p2wsh_sum.csv"),
  fetchCSV("/csv/inputs_p2tr_keypath_sum.csv"),
  fetchCSV("/csv/inputs_p2tr_scriptpath_sum.csv"),
  fetchCSV("/csv/inputs_p2a_sum.csv"),
  fetchCSV("/csv/inputs_unknown_sum.csv"),
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
    p2tr_keypath: [],
    p2tr_scriptpath: [],
    p2a: [],
    unknown: []
  }

  // CSV indices
  const [dates,
    inputs_p2pk, inputs_p2pkh, inputs_p2wpkh, inputs_p2ms, inputs_p2sh, inputs_p2wsh,
    inputs_p2tr_keypath, inputs_p2tr_scriptpath, inputs_p2a, inputs_unknown] = input;

  // Initialize cumulative counters
  let cumulative_p2pk = 0;
  let cumulative_p2pkh = 0;
  let cumulative_p2wpkh = 0;
  let cumulative_p2ms = 0;
  let cumulative_p2sh = 0;
  let cumulative_p2wsh = 0;
  let cumulative_p2tr_keypath = 0;
  let cumulative_p2tr_scriptpath = 0;
  let cumulative_p2a = 0;
  let cumulative_unknown = 0;

  for (let i = 0; i < dates.length; i++) {
    data.date.push(+(new Date(dates[i].date)))

    // Calculate cumulative sums
    cumulative_p2pk += parseFloat(inputs_p2pk[i]["inputs_p2pk_sum"]) || 0
    cumulative_p2pkh += parseFloat(inputs_p2pkh[i]["inputs_p2pkh_sum"]) || 0
    cumulative_p2wpkh += parseFloat(inputs_p2wpkh[i]["inputs_p2wpkh_sum"]) || 0
    cumulative_p2ms += parseFloat(inputs_p2ms[i]["inputs_p2ms_sum"]) || 0
    cumulative_p2sh += parseFloat(inputs_p2sh[i]["inputs_p2sh_sum"]) || 0
    cumulative_p2wsh += parseFloat(inputs_p2wsh[i]["inputs_p2wsh_sum"]) || 0
    cumulative_p2tr_keypath += parseFloat(inputs_p2tr_keypath[i]["inputs_p2tr_keypath_sum"]) || 0
    cumulative_p2tr_scriptpath += parseFloat(inputs_p2tr_scriptpath[i]["inputs_p2tr_scriptpath_sum"]) || 0
    cumulative_p2a += parseFloat(inputs_p2a[i]["inputs_p2a_sum"]) || 0
    cumulative_unknown += parseFloat(inputs_unknown[i]["inputs_unknown_sum"]) || 0

    data.p2pk.push(cumulative_p2pk)
    data.p2pkh.push(cumulative_p2pkh)
    data.p2wpkh.push(cumulative_p2wpkh)
    data.p2ms.push(cumulative_p2ms)
    data.p2sh.push(cumulative_p2sh)
    data.p2wsh.push(cumulative_p2wsh)
    data.p2tr_keypath.push(cumulative_p2tr_keypath)
    data.p2tr_scriptpath.push(cumulative_p2tr_scriptpath)
    data.p2a.push(cumulative_p2a)
    data.unknown.push(cumulative_unknown)
  }

  return data
}

const DATA_KEYS = ['p2pk', 'p2pkh', 'p2wpkh', 'p2ms', 'p2sh', 'p2wsh', 'p2tr_keypath', 'p2tr_scriptpath', 'p2a', 'unknown']

function chartDefinition(d, movingAverage) {
  let option = multiLineChart(d, DATA_KEYS, NAMES, movingAverage, PRECISION, START_DATE, ANNOTATIONS);
  option.tooltip["order"] = "valueDesc";
  option.tooltip["valueFormatter"] = (v) => Math.round(v).toLocaleString();
  option.yAxis.axisLabel = {formatter: (v) => formatCount(v)};
  return option;
}
