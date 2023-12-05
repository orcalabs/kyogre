import xgboost as xgb
import pandas as pd
from sklearn.model_selection import train_test_split
from io import StringIO

CAT = True

def train(model, training_data, num_train_rounds, use_gpu, test_fraction):
    input = pd.read_json(StringIO(training_data), orient='records')

    cats = ['species_group_id']
    for col in cats:
        input[col] = input[col].astype('category')

    params = {"objective":"reg:squarederror"}
    if use_gpu == True:
        params["device"] = "cuda"
    else:
        params["tree_method"] = "hist"

    if test_fraction is None:
        labels = input['weight']
        train = input.drop('weight',axis=1)
        dtrain_reg = xgb.DMatrix(train, label=labels,enable_categorical=CAT)
        return (xgb.train(
            params,
            dtrain_reg,
            num_boost_round=num_train_rounds,
            xgb_model = model).save_raw(raw_format="ubj"), None)

    else:
        x_train, x_test, y_train, y_test = train_test_split(input.drop('weight',axis=1), input['weight'], test_size=0.2, random_state=42)
        dtest_reg = xgb.DMatrix(x_test, label=y_test,enable_categorical=CAT)
        dtrain_reg = xgb.DMatrix(x_train, label=y_train,enable_categorical=CAT)
        val = xgb.train(
            params,
            dtrain_reg,
            num_boost_round=num_train_rounds,
            evals = [(dtest_reg, "test")], verbose_eval=100,
            early_stopping_rounds = 100,
            xgb_model = model)

        return (val.save_raw(raw_format="ubj"), val.best_score)

def predict(model, prediction_data):
    loaded_model = xgb.Booster(model_file=model)
    input = pd.read_json(StringIO(prediction_data), orient='records')
    cats = ['species_group_id']
    for col in cats:
        input[col] = input[col].astype('category')

    data = xgb.DMatrix(input, enable_categorical=CAT)
    preds = loaded_model.predict(data)
    return preds.tolist()

if __name__ == "__main__":
    data = "INSERT_TEST_JSON_DATA"
    train(None, data, 1, True, 0.2)
