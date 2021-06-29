from sklearn.datasets import fetch_20newsgroups
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.naive_bayes import MultinomialNB
from sklearn import metrics

from . import Benchmark

categories = [
    'alt.atheism', 'talk.religion.misc',
    'comp.graphics', 'sci.space'
]

newsgroups_train = fetch_20newsgroups(subset='train', categories=categories,
                                      remove=('headers', 'footers', 'quotes'))
newsgroups_test = fetch_20newsgroups(subset='test', categories=categories,
                                     remove=('headers', 'footers', 'quotes'))

vectorizer = TfidfVectorizer()
vectors = vectorizer.fit_transform(newsgroups_train.data)
vectors_test = vectorizer.transform(newsgroups_test.data)


class MachineLearningBenchmark(Benchmark):
    def __init__(self):
        self.X = vectors
        self.y = newsgroups_train.target
        self.classes = list(set(self.y))

        self.batch_size = 32
        self.last_batch = self.X.shape[0] // self.batch_size

        self.reset()

    @property
    def name(self):
        return 'Machine learning'

    def run_iteration(self):
        # Compute batch indices
        start = self.batch_num * self.batch_size
        end = (self.batch_num + 1) * self.batch_size

        # Extract batch from dataset
        X_batch = self.X[start:end]
        y_batch = self.y[start:end]

        # Train on batch
        self.clf.partial_fit(X_batch, y_batch, self.classes)

        # Move on to next batch
        self.batch_num += 1

    def verify_result(self):
        pred = self.clf.predict(vectors_test)
        f1_score = metrics.f1_score(
            newsgroups_test.target, pred, average='macro')
        assert f1_score > 0.50

    @property
    def done(self):
        return self.batch_num == self.last_batch

    def reset(self):
        self.clf = MultinomialNB(alpha=.01)
        self.batch_num = 0
